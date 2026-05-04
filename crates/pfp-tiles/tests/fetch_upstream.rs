//! Integration tests for `fetch::fetch_upstream_url` against an httpmock
//! server. Verifies User-Agent, conditional GET, 7-day floor on
//! Cache-Control max-age, 304 Not Modified handling, and 5xx surfacing.

use httpmock::prelude::*;
use pfp_tiles::{build_osm_client, fetch, sidecar::TileMeta, TileError};

#[tokio::test]
async fn user_agent_includes_contact_email() {
    let server = MockServer::start_async().await;
    let expected_ua = format!(
        "photo-foot-print/{} (+contact: jykim.loa2000@gmail.com)",
        env!("CARGO_PKG_VERSION")
    );
    let m = server
        .mock_async(|when, then| {
            when.method(GET)
                .path("/14/13708/6334.png")
                .header("user-agent", expected_ua.as_str());
            then.status(200).body(b"_tile_bytes");
        })
        .await;
    let client = build_osm_client().unwrap();
    let url = format!("{}/14/13708/6334.png", server.base_url());
    let fresh = fetch::fetch_upstream_url(&client, &url, None)
        .await
        .unwrap();
    assert_eq!(fresh.bytes, b"_tile_bytes");
    m.assert_async().await;
}

#[tokio::test]
async fn conditional_get_sends_if_none_match_when_etag_known() {
    let server = MockServer::start_async().await;
    let m = server
        .mock_async(|when, then| {
            when.method(GET)
                .path("/5/1/1.png")
                .header("if-none-match", "\"abc\"");
            then.status(304);
        })
        .await;
    let client = build_osm_client().unwrap();
    let url = format!("{}/5/1/1.png", server.base_url());
    let prev = TileMeta {
        schema_version: 1,
        max_age_secs: 7 * 86_400,
        fetched_at_unix: 0,
        etag: Some("\"abc\"".to_string()),
        last_modified: None,
    };
    let result = fetch::fetch_upstream_url(&client, &url, Some(&prev)).await;
    assert!(matches!(result, Err(TileError::UpstreamStatus(304))));
    m.assert_async().await;
}

#[tokio::test]
async fn conditional_get_sends_if_modified_since_when_last_modified_known() {
    let server = MockServer::start_async().await;
    let lm = "Wed, 21 Oct 2025 07:28:00 GMT";
    let m = server
        .mock_async(|when, then| {
            when.method(GET)
                .path("/9/9/9.png")
                .header("if-modified-since", lm);
            then.status(304);
        })
        .await;
    let client = build_osm_client().unwrap();
    let url = format!("{}/9/9/9.png", server.base_url());
    let prev = TileMeta {
        schema_version: 1,
        max_age_secs: 7 * 86_400,
        fetched_at_unix: 0,
        etag: None,
        last_modified: Some(lm.to_string()),
    };
    let result = fetch::fetch_upstream_url(&client, &url, Some(&prev)).await;
    assert!(matches!(result, Err(TileError::UpstreamStatus(304))));
    m.assert_async().await;
}

#[tokio::test]
async fn server_max_age_below_seven_days_is_floored() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(GET).path("/3/2/1.png");
            then.status(200)
                .header("Cache-Control", "public, max-age=60")
                .body(b"_x");
        })
        .await;
    let client = build_osm_client().unwrap();
    let url = format!("{}/3/2/1.png", server.base_url());
    let fresh = fetch::fetch_upstream_url(&client, &url, None)
        .await
        .unwrap();
    assert!(
        fresh.meta.max_age_secs >= 7 * 86_400,
        "expected >= 7d floor, got {}",
        fresh.meta.max_age_secs
    );
}

#[tokio::test]
async fn server_max_age_above_seven_days_is_preserved() {
    let server = MockServer::start_async().await;
    let two_weeks = 14 * 86_400;
    server
        .mock_async(|when, then| {
            when.method(GET).path("/4/4/4.png");
            then.status(200)
                .header("Cache-Control", &format!("public, max-age={two_weeks}"))
                .body(b"_y");
        })
        .await;
    let client = build_osm_client().unwrap();
    let url = format!("{}/4/4/4.png", server.base_url());
    let fresh = fetch::fetch_upstream_url(&client, &url, None)
        .await
        .unwrap();
    assert_eq!(fresh.meta.max_age_secs, two_weeks);
}

#[tokio::test]
async fn upstream_5xx_surfaces_as_upstream_status_error() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(GET).path("/0/0/0.png");
            then.status(503).body(b"");
        })
        .await;
    let client = build_osm_client().unwrap();
    let url = format!("{}/0/0/0.png", server.base_url());
    let result = fetch::fetch_upstream_url(&client, &url, None).await;
    assert!(matches!(result, Err(TileError::UpstreamStatus(503))));
}

#[tokio::test]
async fn etag_and_last_modified_captured_on_200() {
    let server = MockServer::start_async().await;
    server
        .mock_async(|when, then| {
            when.method(GET).path("/2/2/2.png");
            then.status(200)
                .header("ETag", "\"deadbeef\"")
                .header("Last-Modified", "Wed, 21 Oct 2025 07:28:00 GMT")
                .body(b"_z");
        })
        .await;
    let client = build_osm_client().unwrap();
    let url = format!("{}/2/2/2.png", server.base_url());
    let fresh = fetch::fetch_upstream_url(&client, &url, None)
        .await
        .unwrap();
    assert_eq!(fresh.meta.etag.as_deref(), Some("\"deadbeef\""));
    assert_eq!(
        fresh.meta.last_modified.as_deref(),
        Some("Wed, 21 Oct 2025 07:28:00 GMT")
    );
}
