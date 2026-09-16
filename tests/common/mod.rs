#![allow(dead_code)]

use axum::{body::Bytes, http::StatusCode, Router};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

pub async fn test_app() -> Router {
    let pool = local_kms::db::connect_in_memory()
        .await
        .expect("failed to open in-memory sqlite pool");
    local_kms::db::migrate(&pool).await.expect("failed to run migrations");
    local_kms::app(pool)
}

/// Sends a `POST /` request with the given `X-Amz-Target` action suffix (e.g. "CreateKey")
/// and JSON body, and returns the status code, the `X-Amzn-ErrorType` header (if any), and the
/// parsed JSON response body (`Value::Null` for an empty body).
pub async fn send(app: &Router, action: &str, body: Value) -> (StatusCode, Option<String>, Value) {
    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/")
        .header("content-type", "application/x-amz-json-1.1")
        .header("x-amz-target", format!("TrentService.{action}"))
        .body(axum::body::Body::from(body.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let error_type = response
        .headers()
        .get("x-amzn-errortype")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let bytes: Bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("response body should be valid JSON")
    };

    (status, error_type, value)
}

/// Sends a raw `POST /` request, letting the caller control headers/body directly (for
/// dispatch-layer edge cases like a missing `X-Amz-Target` header or malformed JSON).
pub async fn send_raw(
    app: &Router,
    target_header: Option<&str>,
    body: &str,
) -> (StatusCode, Option<String>, Value) {
    let mut builder = axum::http::Request::builder()
        .method("POST")
        .uri("/")
        .header("content-type", "application/x-amz-json-1.1");
    if let Some(target) = target_header {
        builder = builder.header("x-amz-target", target);
    }
    let request = builder.body(axum::body::Body::from(body.to_string())).unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let error_type = response
        .headers()
        .get("x-amzn-errortype")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let bytes: Bytes = response.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("response body should be valid JSON")
    };

    (status, error_type, value)
}

pub async fn create_test_key(app: &Router) -> Value {
    let (status, _, body) = send(app, "CreateKey", serde_json::json!({})).await;
    assert_eq!(status, StatusCode::OK, "CreateKey should succeed: {body:?}");
    body["KeyMetadata"].clone()
}

pub async fn create_test_alias(app: &Router, alias_name: &str, target_key_id: &str) {
    let (status, _, body) = send(
        app,
        "CreateAlias",
        serde_json::json!({ "AliasName": alias_name, "TargetKeyId": target_key_id }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "CreateAlias should succeed: {body:?}");
}
