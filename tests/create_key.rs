mod common;

use axum::http::StatusCode;
use common::{send, test_app};
use serde_json::json;

#[tokio::test]
async fn default_response_matches_spec() {
    let app = test_app().await;
    let (status, _, body) = send(&app, "CreateKey", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{body:?}");

    let meta = &body["KeyMetadata"];
    assert_eq!(meta["KeyState"], "Enabled");
    assert_eq!(meta["Enabled"], true);
    assert_eq!(meta["KeyUsage"], "ENCRYPT_DECRYPT");
    assert_eq!(meta["KeySpec"], "SYMMETRIC_DEFAULT");
    assert_eq!(meta["KeyManager"], "CUSTOMER");
    assert_eq!(meta["Origin"], "AWS_KMS");
    assert_eq!(meta["MultiRegion"], false);
    assert_eq!(meta["EncryptionAlgorithms"], json!(["SYMMETRIC_DEFAULT"]));
    assert_eq!(meta["AWSAccountId"], "111122223333");
    assert_eq!(meta["Description"], "");

    let key_id = meta["KeyId"].as_str().unwrap();
    uuid::Uuid::parse_str(key_id).expect("KeyId should be a UUID");

    let arn = meta["Arn"].as_str().unwrap();
    assert_eq!(arn, format!("arn:aws:kms:us-east-2:111122223333:key/{key_id}"));

    let material_id = meta["CurrentKeyMaterialId"].as_str().unwrap();
    assert_eq!(material_id.len(), 64);
    assert!(material_id.chars().all(|c| c.is_ascii_hexdigit()));

    assert!(meta["CreationDate"].is_number());
}

#[tokio::test]
async fn description_is_echoed_back() {
    let app = test_app().await;
    let (status, _, body) = send(&app, "CreateKey", json!({ "Description": "for envelope encryption" })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["KeyMetadata"]["Description"], "for envelope encryption");
}

#[tokio::test]
async fn unsupported_key_spec_is_rejected() {
    let app = test_app().await;
    let (status, error_type, _) = send(&app, "CreateKey", json!({ "KeySpec": "RSA_2048" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("UnsupportedOperationException"));
}

#[tokio::test]
async fn unsupported_key_usage_is_rejected() {
    let app = test_app().await;
    let (status, error_type, _) = send(&app, "CreateKey", json!({ "KeyUsage": "SIGN_VERIFY" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("UnsupportedOperationException"));
}

#[tokio::test]
async fn unsupported_origin_is_rejected() {
    let app = test_app().await;
    let (status, error_type, _) = send(&app, "CreateKey", json!({ "Origin": "EXTERNAL" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("UnsupportedOperationException"));
}

#[tokio::test]
async fn repeated_calls_produce_distinct_keys() {
    let app = test_app().await;
    let (_, _, a) = send(&app, "CreateKey", json!({})).await;
    let (_, _, b) = send(&app, "CreateKey", json!({})).await;

    assert_ne!(a["KeyMetadata"]["KeyId"], b["KeyMetadata"]["KeyId"]);
    assert_ne!(a["KeyMetadata"]["Arn"], b["KeyMetadata"]["Arn"]);
    assert_ne!(
        a["KeyMetadata"]["CurrentKeyMaterialId"],
        b["KeyMetadata"]["CurrentKeyMaterialId"]
    );
}
