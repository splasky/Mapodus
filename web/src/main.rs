mod api;
mod session;

use axum::Router;
use axum::response::Response;
use axum::routing::get;
use axum::http::StatusCode;
use rust_embed::RustEmbed;
use tower_http::cors::CorsLayer;
use tower_sessions::{MemoryStore, SessionManagerLayer};

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

#[tokio::main]
async fn main() {
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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind to port 3000");

    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
