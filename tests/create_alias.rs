mod common;

use axum::http::StatusCode;
use common::{create_test_alias, create_test_key, send, test_app};
use serde_json::json;

#[tokio::test]
async fn happy_path_returns_empty_body() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let (status, _, body) = send(
        &app,
        "CreateAlias",
        json!({ "AliasName": "alias/Happy", "TargetKeyId": key_id }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, serde_json::Value::Null, "response body should be empty");
}

#[tokio::test]
async fn duplicate_alias_name_is_rejected() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    create_test_alias(&app, "alias/Dup", key_id).await;
    let (status, error_type, _) = send(
        &app,
        "CreateAlias",
        json!({ "AliasName": "alias/Dup", "TargetKeyId": key_id }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("AlreadyExistsException"));
}

#[tokio::test]
async fn missing_alias_prefix_is_rejected() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let (status, error_type, _) = send(
        &app,
        "CreateAlias",
        json!({ "AliasName": "NotAnAlias", "TargetKeyId": key_id }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("InvalidAliasNameException"));
}

#[tokio::test]
async fn disallowed_characters_are_rejected() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let (status, error_type, _) = send(
        &app,
        "CreateAlias",
        json!({ "AliasName": "alias/has space", "TargetKeyId": key_id }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("InvalidAliasNameException"));
}

#[tokio::test]
async fn reserved_aws_prefix_is_rejected() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();

    let (status, error_type, _) = send(
        &app,
        "CreateAlias",
        json!({ "AliasName": "alias/aws/reserved", "TargetKeyId": key_id }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("InvalidAliasNameException"));
}

#[tokio::test]
async fn unknown_target_key_id_returns_not_found() {
    let app = test_app().await;
    let (status, error_type, _) = send(
        &app,
        "CreateAlias",
        json!({ "AliasName": "alias/Orphan", "TargetKeyId": "does-not-exist" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("NotFoundException"));
}

#[tokio::test]
async fn target_key_id_as_alias_is_rejected() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();
    create_test_alias(&app, "alias/Existing", key_id).await;

    let (status, error_type, _) = send(
        &app,
        "CreateAlias",
        json!({ "AliasName": "alias/PointsAtAlias", "TargetKeyId": "alias/Existing" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(error_type.as_deref(), Some("NotFoundException"));
}

#[tokio::test]
async fn alias_is_immediately_usable_in_generate_data_key() {
    let app = test_app().await;
    let key = create_test_key(&app).await;
    let key_id = key["KeyId"].as_str().unwrap();
    create_test_alias(&app, "alias/UsableNow", key_id).await;

    let (status, _, body) = send(
        &app,
        "GenerateDataKey",
        json!({ "KeyId": "alias/UsableNow", "KeySpec": "AES_256" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body:?}");
}
