use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidCiphertext(String),
    #[error("{0}")]
    IncorrectKey(String),
    #[error("{0}")]
    AlreadyExists(String),
    #[error("{0}")]
    InvalidAliasName(String),
    #[error("{0}")]
    UnsupportedOperation(String),
    #[error("{0}")]
    UnknownOperation(String),
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    fn exception_name(&self) -> &'static str {
        match self {
            AppError::NotFound(_) => "NotFoundException",
            AppError::InvalidCiphertext(_) => "InvalidCiphertextException",
            AppError::IncorrectKey(_) => "IncorrectKeyException",
            AppError::AlreadyExists(_) => "AlreadyExistsException",
            AppError::InvalidAliasName(_) => "InvalidAliasNameException",
            AppError::UnsupportedOperation(_) => "UnsupportedOperationException",
            AppError::UnknownOperation(_) => "UnknownOperationException",
            AppError::Validation(_) => "ValidationException",
            AppError::Internal(_) => "KMSInternalException",
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        }
    }

    fn message(&self) -> &str {
        match self {
            AppError::NotFound(m)
            | AppError::InvalidCiphertext(m)
            | AppError::IncorrectKey(m)
            | AppError::AlreadyExists(m)
            | AppError::InvalidAliasName(m)
            | AppError::UnsupportedOperation(m)
            | AppError::UnknownOperation(m)
            | AppError::Validation(m)
            | AppError::Internal(m) => m,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let name = self.exception_name();
        let status = self.status_code();
        let body = serde_json::json!({
            "__type": format!("com.amazonaws.kms#{name}"),
            "message": self.message(),
        });
        let mut response = (status, Json(body)).into_response();
        if let Ok(value) = name.parse() {
            response.headers_mut().insert("x-amzn-errortype", value);
        }
        response
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}
