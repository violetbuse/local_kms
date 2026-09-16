mod common;

use axum::http::StatusCode;
use common::{create_test_key, send, send_raw, test_app};
use serde_json::json;

#[tokio::test]
async fn create_key_routes_correctly() {
    let app = test_app().await;
    let (status, _, body) = send(&app, "CreateKey", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("KeyMetadata").is_some());
}

#[tokio::test]
async fn create_alias_routes_correctly() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let (status, _, body) = send(
        &app,
        "CreateAlias",
        json!({ "AliasName": "alias/DispatchTest", "TargetKeyId": key_id }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, serde_json::Value::Null);
}

#[tokio::test]
async fn generate_data_key_routes_correctly() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let (status, _, body) = send(
        &app,
        "GenerateDataKey",
        json!({ "KeyId": key_id, "KeySpec": "AES_256" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("CiphertextBlob").is_some());
}

#[tokio::test]
async fn decrypt_routes_correctly() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let (_, _, generated) = send(
        &app,
        "GenerateDataKey",
        json!({ "KeyId": key_id, "KeySpec": "AES_256" }),
    )
    .await;

    let (status, _, body) = send(&app, "Decrypt", json!({ "CiphertextBlob": generated["CiphertextBlob"] })).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("Plaintext").is_some());
    assert!(body.get("CiphertextBlob").is_none());
}

#[tokio::test]
async fn missing_x_amz_target_header_is_rejected() {
    let app = test_app().await;
    let (status, error_type, _) = send_raw(&app, None, "{}").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("ValidationException"));
}

#[tokio::test]
async fn unknown_action_is_rejected() {
    let app = test_app().await;
    let (status, error_type, _) = send_raw(&app, Some("TrentService.FooBar"), "{}").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("UnknownOperationException"));
}

#[tokio::test]
async fn malformed_json_body_is_rejected() {
    let app = test_app().await;
    let (status, error_type, body) = send_raw(&app, Some("TrentService.CreateKey"), "{not json").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("ValidationException"));
    assert!(body["__type"].as_str().unwrap().contains("ValidationException"));
    assert!(body["message"].is_string());
}
