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

use std::collections::HashMap;

use umap_core::google_maps_api::GoogleMapsClient;

#[tokio::test]
async fn get_place_details_skips_google_knowledge_graph_ids() {
    let client = GoogleMapsClient::new(HashMap::new());

    let details = client
        .get_place_details("/g/1tf26dh2")
        .await
        .expect("unsupported IDs should not be sent to Google Maps preview/place");

    assert!(details.is_none());
}

#[tokio::test]
async fn get_place_details_skips_google_topic_ids() {
    let client = GoogleMapsClient::new(HashMap::new());

    let details = client
        .get_place_details("/m/03cyfr9")
        .await
        .expect("unsupported IDs should not be sent to Google Maps preview/place");

    assert!(details.is_none());
}
