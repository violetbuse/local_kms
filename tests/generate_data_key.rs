mod common;

use axum::http::StatusCode;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use common::{create_test_key, send, test_app};

#[tokio::test]
async fn both_key_spec_and_number_of_bytes_is_rejected() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let (status, error_type, _) = send(
        &app,
        "GenerateDataKey",
        serde_json::json!({ "KeyId": key_id, "KeySpec": "AES_256", "NumberOfBytes": 32 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("ValidationException"));
}

#[tokio::test]
async fn neither_key_spec_nor_number_of_bytes_is_rejected() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let (status, error_type, _) = send(&app, "GenerateDataKey", serde_json::json!({ "KeyId": key_id })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("ValidationException"));
}

#[tokio::test]
async fn unknown_key_id_returns_not_found() {
    let app = test_app().await;
    let (status, error_type, _) = send(
        &app,
        "GenerateDataKey",
        serde_json::json!({ "KeyId": "does-not-exist", "KeySpec": "AES_256" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("NotFoundException"));
}

#[tokio::test]
async fn key_spec_aes_128_produces_16_bytes() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let (status, _, body) = send(
        &app,
        "GenerateDataKey",
        serde_json::json!({ "KeyId": key_id, "KeySpec": "AES_128" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let plaintext = STANDARD.decode(body["Plaintext"].as_str().unwrap()).unwrap();
    assert_eq!(plaintext.len(), 16);
}

#[tokio::test]
async fn key_spec_aes_256_produces_32_bytes() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let (status, _, body) = send(
        &app,
        "GenerateDataKey",
        serde_json::json!({ "KeyId": key_id, "KeySpec": "AES_256" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let plaintext = STANDARD.decode(body["Plaintext"].as_str().unwrap()).unwrap();
    assert_eq!(plaintext.len(), 32);
}

#[tokio::test]
async fn number_of_bytes_produces_requested_length() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let (status, _, body) = send(
        &app,
        "GenerateDataKey",
        serde_json::json!({ "KeyId": key_id, "NumberOfBytes": 45 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let plaintext = STANDARD.decode(body["Plaintext"].as_str().unwrap()).unwrap();
    assert_eq!(plaintext.len(), 45);
}

#[tokio::test]
async fn number_of_bytes_out_of_range_is_rejected() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    for n in [0, 2000] {
        let (status, error_type, _) = send(
            &app,
            "GenerateDataKey",
            serde_json::json!({ "KeyId": key_id, "NumberOfBytes": n }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "n={n}");
        assert_eq!(error_type.as_deref(), Some("ValidationException"), "n={n}");
    }
}

#[tokio::test]
async fn response_shape_matches_spec() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();
    let key_arn = key["Arn"].as_str().unwrap();

    let (status, _, body) = send(
        &app,
        "GenerateDataKey",
        serde_json::json!({ "KeyId": key_id, "KeySpec": "AES_256" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let obj = body.as_object().unwrap();
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort();
    assert_eq!(keys, ["CiphertextBlob", "KeyId", "KeyMaterialId", "Plaintext"]);

    assert_eq!(body["KeyId"].as_str().unwrap(), key_arn);
    let material_id = body["KeyMaterialId"].as_str().unwrap();
    assert_eq!(material_id.len(), 64);
    assert!(material_id.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
}
