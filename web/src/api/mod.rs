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

pub mod bookmarks;
pub mod errors;
pub mod external;
pub mod google_import;
pub mod settings;
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
        .route(
            "/api/bookmarks/select",
            axum::routing::post(bookmarks::select),
        )
        .route(
            "/api/bookmarks/auto_enrich",
            axum::routing::post(bookmarks::auto_enrich),
        )
        .route(
            "/api/settings",
            axum::routing::get(settings::get).post(settings::update),
        )
        .route("/api/open-external", axum::routing::post(external::open))
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
        router = router
            .route(
                "/api/google/debug",
                axum::routing::post(google_import::debug_import),
            )
            .route(
                "/api/bookmarks/debug_place_details",
                axum::routing::get(bookmarks::debug_place_details),
            );
    }

    router
}
