use axum::http::StatusCode;
mod common;
use common::send;
use serde_json::json;

/// Keys and aliases must survive a simulated process restart: a fresh connection pool opened
/// on the same on-disk SQLite file should see everything a prior pool wrote.
#[tokio::test]
async fn keys_and_aliases_survive_a_simulated_restart() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("persistence.db");
    let db_path_str = db_path.to_str().unwrap();

    let (key_id, alias_name) = {
        let pool_a = local_kms::db::connect_file(db_path_str).await.unwrap();
        local_kms::db::migrate(&pool_a).await.unwrap();
        let app_a = local_kms::app(pool_a);

        let (status, _, key_body) = send(&app_a, "CreateKey", json!({})).await;
        assert_eq!(status, StatusCode::OK);
        let key_id = key_body["KeyMetadata"]["KeyId"].as_str().unwrap().to_string();

        let alias_name = "alias/SurvivesRestart".to_string();
        let (status, _, _) = send(
            &app_a,
            "CreateAlias",
            json!({ "AliasName": alias_name, "TargetKeyId": key_id }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);

        (key_id, alias_name)
        // pool_a/app_a dropped here, simulating process shutdown
    };

    let pool_b = local_kms::db::connect_file(db_path_str).await.unwrap();
    // No migrate call: the schema should already exist on disk from pool_a's run.
    let app_b = local_kms::app(pool_b);

    let (status, _, gen_body) = send(
        &app_b,
        "GenerateDataKey",
        json!({ "KeyId": alias_name, "KeySpec": "AES_256" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{gen_body:?}");
    assert!(gen_body["KeyId"].as_str().unwrap().contains(&key_id));
}
