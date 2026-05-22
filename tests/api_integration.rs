//! HTTP integration tests (no full model load for non-English paths).

use salvo::conn::TcpListener;
use salvo::prelude::*;
use serde_json::json;
use std::time::Duration;
use trypanophobe::{app_service, DetectorSlot};

async fn spawn_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let slot = DetectorSlot::new();
    let service = app_service(slot, vec!["*".into()]);
    let acceptor = TcpListener::new("127.0.0.1:0").bind().await;
    let addr = acceptor.local_addr().unwrap();
    let base = format!("http://{addr}");

    let handle = tokio::spawn(async move {
        Server::new(acceptor).serve(service).await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    (base, handle)
}

#[tokio::test]
async fn integration_version_and_redirect() {
    let (base, server) = spawn_test_server().await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let version = client
        .get(format!("{base}/api/version"))
        .send()
        .await
        .unwrap();
    assert!(version.status().is_success());
    let body: serde_json::Value = version.json().await.unwrap();
    assert_eq!(body["name"], "trypanophobe");

    let root = client
        .get(format!("{base}/"))
        .send()
        .await
        .unwrap();
    assert_eq!(root.status(), reqwest::StatusCode::SEE_OTHER);
    let location = root.headers().get(reqwest::header::LOCATION).unwrap();
    assert!(location.to_str().unwrap().contains("/swagger-ui"));

    server.abort();
}

#[tokio::test]
async fn integration_check_non_english() {
    let (base, server) = spawn_test_server().await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let res = client
        .post(format!("{base}/api/check"))
        .json(&json!({ "text": "Bonjour le monde" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["label"], "REJECTED");
    assert_eq!(body["rejected"], true);

    server.abort();
}

#[tokio::test]
async fn integration_check_empty_text() {
    let (base, server) = spawn_test_server().await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let res = client
        .post(format!("{base}/api/check"))
        .json(&json!({ "text": "  " }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);

    server.abort();
}
