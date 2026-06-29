pub mod auth;
pub mod bookmarks;
pub mod errors;
pub mod google_import;
pub mod umap;

use axum::Router;

pub fn routes() -> Router {
    Router::new()
        .route("/api/auth/google", axum::routing::get(auth::google_login))
        .route("/api/auth/google/callback", axum::routing::get(auth::google_callback))
        .route("/api/auth/status", axum::routing::get(auth::status))
        .route("/api/bookmarks/upload", axum::routing::post(bookmarks::upload))
        .route("/api/bookmarks", axum::routing::get(bookmarks::list))
        .route("/api/umap/connect", axum::routing::post(umap::connect))
        .route("/api/umap/status", axum::routing::get(umap::status))
        .route("/api/transfer", axum::routing::post(umap::transfer))
        .route("/api/google/import", axum::routing::post(google_import::import))
        .route("/api/google/confirm", axum::routing::post(google_import::confirm))
        .route("/api/google/debug", axum::routing::post(google_import::debug_import))
}
