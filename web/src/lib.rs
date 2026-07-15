// Copyright 2026 HYChang
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod api;
pub mod session;
pub mod settings;

use std::net::SocketAddr;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use rust_embed::RustEmbed;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tower_http::cors::CorsLayer;
use tower_sessions::{MemoryStore, SessionManagerLayer};
use umap_core::google_maps_api::GoogleMapsClient;

#[derive(RustEmbed)]
#[folder = "static"]
struct StaticFiles;

pub fn build_app() -> Router {
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    Router::new()
        .route("/", get(index_handler))
        .route("/{*path}", get(static_handler))
        .merge(api::routes())
        .layer(session_layer)
        .layer(CorsLayer::permissive())
        // Google Takeout CSV exports can exceed Axum's 2 MiB default body limit.
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024))
}

pub async fn serve_listener(listener: TcpListener) -> std::io::Result<()> {
    axum::serve(listener, build_app()).await
}

pub async fn serve_on_available_port() -> std::io::Result<(SocketAddr, JoinHandle<()>)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        if let Err(error) = serve_listener(listener).await {
            eprintln!("Desktop backend failed: {}", error);
        }
    });

    Ok((addr, handle))
}

pub async fn run_dev_import(cookies_str: &str) {
    println!("=== Dev Mode: Google Maps Import ===");
    let cookies = parse_cookies(cookies_str);
    println!("Cookies: {} keys parsed", cookies.len());
    for key in cookies.keys() {
        println!("  {}: ****", key);
    }

    let client = GoogleMapsClient::new(cookies);
    match client.get_all_saved_places().await {
        Ok(places) => {
            println!("\nSuccess! Imported {} places total.\n", places.len());

            let mut list_map: std::collections::BTreeMap<
                String,
                Vec<&umap_core::google_maps_api::GoogleSavedPlace>,
            > = std::collections::BTreeMap::new();
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
        None => match StaticFiles::get("index.html") {
            Some(content) => Response::builder()
                .header("Content-Type", "text/html")
                .body(axum::body::Body::from(content.data))
                .unwrap(),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(axum::body::Body::from("Not found"))
                .unwrap(),
        },
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
