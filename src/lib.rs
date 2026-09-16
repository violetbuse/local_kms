pub mod actions;
pub mod crypto;
pub mod db;
pub mod error;
pub mod keyid;

use axum::{
    body::Bytes,
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use sqlx::SqlitePool;

use error::AppError;

pub fn app(pool: SqlitePool) -> Router {
    Router::new().route("/", post(handle)).with_state(pool)
}

async fn handle(headers: HeaderMap, State(pool): State<SqlitePool>, body: Bytes) -> Response {
    let target = match headers.get("x-amz-target").and_then(|v| v.to_str().ok()) {
        Some(t) if !t.is_empty() => t,
        _ => return AppError::Validation("Missing X-Amz-Target header".into()).into_response(),
    };

    let action = target.rsplit('.').next().unwrap_or(target);

    let result = match action {
        "CreateKey" => actions::create_key(&pool, &body).await,
        "CreateAlias" => actions::create_alias(&pool, &body).await,
        "GenerateDataKey" => actions::generate_data_key(&pool, &body).await,
        "Decrypt" => actions::decrypt(&pool, &body).await,
        other => Err(AppError::UnknownOperation(format!(
            "The requested operation '{other}' is not recognized"
        ))),
    };

    match result {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}
