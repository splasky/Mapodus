mod api;
mod session;

use axum::Router;
use axum::response::Response;
use axum::routing::get;
use axum::http::StatusCode;
use clap::Parser;
use rust_embed::RustEmbed;
use tower_http::cors::CorsLayer;
use tower_sessions::{MemoryStore, SessionManagerLayer};
use umap_core::google_maps_api::GoogleMapsClient;

#[derive(Parser)]
#[command(name = "web", about = "google-maps-to-umap web server")]
struct Args {
    #[arg(
        long,
        help = "Google Maps cookies for dev mode (e.g. 'SAPISID=xxx; SID=yyy; HSID=zzz'). Imports and prints saved lists on startup."
    )]
    google_cookies: Option<String>,
}

#[derive(RustEmbed)]
#[folder = "static"]
struct StaticFiles;

fn serve_static(path: &str) -> Response {
    let filename = if path.is_empty() || path == "/" {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };

    match StaticFiles::get(filename) {
        Some(content) => {
            let mime = mime_guess::from_path(filename).first_or_octet_stream();
            Response::builder()
                .header("Content-Type", mime.as_ref())
                .body(axum::body::Body::from(content.data))
                .unwrap()
        }
        None => {
            match StaticFiles::get("index.html") {
                Some(content) => Response::builder()
                    .header("Content-Type", "text/html")
                    .body(axum::body::Body::from(content.data))
                    .unwrap(),
                None => Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(axum::body::Body::from("Not found"))
                    .unwrap(),
            }
        }
    }
}

async fn static_handler(path: axum::extract::Path<String>) -> Response {
    serve_static(&path)
}

async fn index_handler() -> Response {
    serve_static("index.html")
}

fn parse_cookies(text: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for part in text.split(';') {
        let eq = part.find('=');
        if let Some(pos) = eq {
            let key = part[..pos].trim().to_string();
            let val = part[pos + 1..].trim().to_string();
            if !key.is_empty() && !val.is_empty() {
                map.insert(key, val);
            }
        }
    }
    map
}

async fn run_dev_import(cookies_str: &str) {
    println!("=== Dev Mode: Google Maps Import ===");
    let cookies = parse_cookies(cookies_str);
    println!("Cookies: {} keys parsed", cookies.len());
    for key in cookies.keys() {
        println!("  {}: ****", key);
    }

    let client = GoogleMapsClient::new(cookies);
    match client.collect_all().await {
        Ok(places) => {
            println!("\nSuccess! Imported {} places total.\n", places.len());

            let mut list_map: std::collections::BTreeMap<String, Vec<&umap_core::google_maps_api::GoogleSavedPlace>> = std::collections::BTreeMap::new();
            for place in &places {
                list_map.entry(place.list.clone()).or_default().push(place);
            }

            for (list_name, list_places) in &list_map {
                println!("── {} ({} places) ──", list_name, list_places.len());
                for place in list_places {
                    let latlon = match (place.latitude, place.longitude) {
                        (Some(lat), Some(lon)) => format!("{:.6}, {:.6}", lat, lon),
                        _ => "no coords".to_string(),
                    };
                    let title = place.title.as_deref().unwrap_or("(untitled)");
                    println!("  {:<50} {}", title, latlon);
                }
                println!();
            }
        }
        Err(e) => {
            eprintln!("\nError: {}", e);
            eprintln!("\nMake sure your cookies are valid and include SAPISID, SID, and HSID.");
        }
    }
    println!("=== End Dev Import ===");
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if let Some(ref cookies) = args.google_cookies {
        run_dev_import(cookies).await;
    }

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false);

    let api_routes = api::routes();

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/{*path}", get(static_handler))
        .merge(api_routes)
        .layer(session_layer)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8900")
        .await
        .expect("Failed to bind to port 8900");

    println!("Server running on http://localhost:8900");
    axum::serve(listener, app).await.unwrap();
}
