mod common;

use axum::http::StatusCode;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use common::{create_test_alias, create_test_key, send, test_app};

async fn roundtrip_with_key_id(key_id: &str) {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let raw_key_id = key["KeyId"].as_str().unwrap();
    let key_arn = key["Arn"].as_str().unwrap();

    let key_id = match key_id {
        "raw" => raw_key_id.to_string(),
        "arn" => key_arn.to_string(),
        "alias" => {
            create_test_alias(&app, "alias/RoundtripAlias", raw_key_id).await;
            "alias/RoundtripAlias".to_string()
        }
        "alias_arn" => {
            create_test_alias(&app, "alias/RoundtripAliasArn", raw_key_id).await;
            format!("arn:aws:kms:us-east-2:111122223333:alias/RoundtripAliasArn")
        }
        _ => unreachable!(),
    };

    let (status, _, gen_body) = send(
        &app,
        "GenerateDataKey",
        serde_json::json!({ "KeyId": key_id, "KeySpec": "AES_256" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "GenerateDataKey should succeed: {gen_body:?}");

    let ciphertext_blob = gen_body["CiphertextBlob"].as_str().unwrap();
    let expected_plaintext = STANDARD.decode(gen_body["Plaintext"].as_str().unwrap()).unwrap();

    let (status, _, dec_body) = send(
        &app,
        "Decrypt",
        serde_json::json!({ "CiphertextBlob": ciphertext_blob }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "Decrypt should succeed: {dec_body:?}");

    let actual_plaintext = STANDARD.decode(dec_body["Plaintext"].as_str().unwrap()).unwrap();
    assert_eq!(actual_plaintext, expected_plaintext);
}

#[tokio::test]
async fn roundtrip_by_key_id() {
    roundtrip_with_key_id("raw").await;
}

#[tokio::test]
async fn roundtrip_by_key_arn() {
    roundtrip_with_key_id("arn").await;
}

#[tokio::test]
async fn roundtrip_by_alias() {
    roundtrip_with_key_id("alias").await;
}

#[tokio::test]
async fn roundtrip_by_alias_arn() {
    roundtrip_with_key_id("alias_arn").await;
}
