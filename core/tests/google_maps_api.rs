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
