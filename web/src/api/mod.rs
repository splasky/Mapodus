pub mod bookmarks;
pub mod errors;
pub mod google_import;
pub mod umap;

use axum::Router;

pub fn routes() -> Router {
    let mut router = Router::new()
        .route(
            "/api/bookmarks/upload",
            axum::routing::post(bookmarks::upload),
        )
        .route("/api/bookmarks", axum::routing::get(bookmarks::list))
        .route(
            "/api/bookmarks/enrich",
            axum::routing::post(bookmarks::enrich),
        )
        .route("/api/umap/connect", axum::routing::post(umap::connect))
        .route("/api/umap/status", axum::routing::get(umap::status))
        .route("/api/transfer", axum::routing::post(umap::transfer))
        .route(
            "/api/google/import",
            axum::routing::post(google_import::import),
        )
        .route(
            "/api/google/confirm",
            axum::routing::post(google_import::confirm),
        );

    if std::env::var("DEV_MODE").as_deref() == Ok("true") {
        router = router.route(
            "/api/google/debug",
            axum::routing::post(google_import::debug_import),
        );
    }

    router
}
