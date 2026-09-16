mod common;

use axum::http::StatusCode;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use common::{create_test_key, send, test_app};
use serde_json::json;

async fn generate(app: &axum::Router, key_id: &str, context: Option<serde_json::Value>) -> serde_json::Value {
    let mut body = json!({ "KeyId": key_id, "KeySpec": "AES_256" });
    if let Some(ctx) = context {
        body["EncryptionContext"] = ctx;
    }
    let (status, _, resp) = send(app, "GenerateDataKey", body).await;
    assert_eq!(status, StatusCode::OK, "GenerateDataKey should succeed: {resp:?}");
    resp
}

#[tokio::test]
async fn matching_encryption_context_round_trips() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let generated = generate(&app, key_id, Some(json!({ "purpose": "test" }))).await;
    let (status, _, dec) = send(
        &app,
        "Decrypt",
        json!({
            "CiphertextBlob": generated["CiphertextBlob"],
            "EncryptionContext": { "purpose": "test" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{dec:?}");
    assert_eq!(dec["Plaintext"], generated["Plaintext"]);
}

#[tokio::test]
async fn omitted_encryption_context_on_decrypt_fails() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let generated = generate(&app, key_id, Some(json!({ "purpose": "test" }))).await;
    let (status, error_type, _) = send(
        &app,
        "Decrypt",
        json!({ "CiphertextBlob": generated["CiphertextBlob"] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("InvalidCiphertextException"));
}

#[tokio::test]
async fn mismatched_encryption_context_fails() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let generated = generate(&app, key_id, Some(json!({ "purpose": "test" }))).await;
    let (status, error_type, _) = send(
        &app,
        "Decrypt",
        json!({
            "CiphertextBlob": generated["CiphertextBlob"],
            "EncryptionContext": { "purpose": "different" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("InvalidCiphertextException"));
}

#[tokio::test]
async fn tampered_ciphertext_fails() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let generated = generate(&app, key_id, None).await;
    let mut blob = STANDARD.decode(generated["CiphertextBlob"].as_str().unwrap()).unwrap();
    let last = blob.len() - 1;
    blob[last] ^= 0xFF;
    let tampered = STANDARD.encode(&blob);

    let (status, error_type, _) = send(&app, "Decrypt", json!({ "CiphertextBlob": tampered })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("InvalidCiphertextException"));
}

#[tokio::test]
async fn non_base64_ciphertext_fails() {
    let app = test_app().await;
    let (status, error_type, _) = send(&app, "Decrypt", json!({ "CiphertextBlob": "not valid base64!!" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("InvalidCiphertextException"));
}

#[tokio::test]
async fn correct_explicit_key_id_succeeds() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let generated = generate(&app, key_id, None).await;
    let (status, _, dec) = send(
        &app,
        "Decrypt",
        json!({ "CiphertextBlob": generated["CiphertextBlob"], "KeyId": key_id }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{dec:?}");
}

#[tokio::test]
async fn wrong_explicit_key_id_returns_incorrect_key() {
    let app = test_app().await;
    let key_a = create_test_key(&app).await;
    let key_b = create_test_key(&app).await;

    let generated = generate(&app, key_a["KeyId"].as_str().unwrap(), None).await;
    let (status, error_type, _) = send(
        &app,
        "Decrypt",
        json!({ "CiphertextBlob": generated["CiphertextBlob"], "KeyId": key_b["KeyId"] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("IncorrectKeyException"));
}

#[tokio::test]
async fn omitted_key_id_resolves_from_blob() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let generated = generate(&app, key_id, None).await;
    let (status, _, dec) = send(&app, "Decrypt", json!({ "CiphertextBlob": generated["CiphertextBlob"] })).await;
    assert_eq!(status, StatusCode::OK, "{dec:?}");
}

#[tokio::test]
async fn response_shape_matches_spec() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();
    let key_arn = key["Arn"].as_str().unwrap();

    let generated = generate(&app, key_id, None).await;
    let (status, _, dec) = send(&app, "Decrypt", json!({ "CiphertextBlob": generated["CiphertextBlob"] })).await;
    assert_eq!(status, StatusCode::OK);

    let obj = dec.as_object().unwrap();
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort();
    assert_eq!(keys, ["EncryptionAlgorithm", "KeyId", "KeyMaterialId", "Plaintext"]);
    assert_eq!(dec["EncryptionAlgorithm"], "SYMMETRIC_DEFAULT");
    assert_eq!(dec["KeyId"], key_arn);
}
