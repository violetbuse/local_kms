use std::collections::BTreeMap;

use axum::{
    body::Bytes,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::crypto;
use crate::db::{self, now_epoch};
use crate::error::AppError;
use crate::keyid::{self, key_arn};

fn parse_body<T: DeserializeOwned>(body: &Bytes) -> Result<T, AppError> {
    serde_json::from_slice(body).map_err(|e| AppError::Validation(format!("invalid request body: {e}")))
}

// ---------- CreateKey ----------

#[derive(Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
struct CreateKeyRequest {
    description: Option<String>,
    key_spec: Option<String>,
    key_usage: Option<String>,
    origin: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct KeyMetadata {
    #[serde(rename = "AWSAccountId")]
    aws_account_id: String,
    arn: String,
    creation_date: i64,
    current_key_material_id: String,
    description: String,
    enabled: bool,
    encryption_algorithms: Vec<String>,
    key_id: String,
    key_manager: String,
    key_spec: String,
    key_state: String,
    key_usage: String,
    multi_region: bool,
    origin: String,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct CreateKeyResponse {
    key_metadata: KeyMetadata,
}

pub async fn create_key(pool: &SqlitePool, body: &Bytes) -> Result<Response, AppError> {
    let req: CreateKeyRequest = parse_body(body)?;

    if let Some(spec) = &req.key_spec {
        if spec != "SYMMETRIC_DEFAULT" {
            return Err(AppError::UnsupportedOperation(format!(
                "KeySpec '{spec}' is not supported by this mock; only SYMMETRIC_DEFAULT is"
            )));
        }
    }
    if let Some(usage) = &req.key_usage {
        if usage != "ENCRYPT_DECRYPT" {
            return Err(AppError::UnsupportedOperation(format!(
                "KeyUsage '{usage}' is not supported by this mock; only ENCRYPT_DECRYPT is"
            )));
        }
    }
    if let Some(origin) = &req.origin {
        if origin != "AWS_KMS" {
            return Err(AppError::UnsupportedOperation(format!(
                "Origin '{origin}' is not supported by this mock; only AWS_KMS is"
            )));
        }
    }

    let key_id = uuid::Uuid::new_v4().to_string();
    let key_material = crypto::random_bytes(32);
    let key_material_id = crypto::hex_id();
    let description = req.description.unwrap_or_default();
    let created_at = now_epoch();

    db::insert_key(
        pool,
        &db::Key {
            key_id: key_id.clone(),
            key_material,
            key_material_id: key_material_id.clone(),
            description: description.clone(),
            created_at,
        },
    )
    .await?;

    let response = CreateKeyResponse {
        key_metadata: KeyMetadata {
            aws_account_id: keyid::FAKE_ACCOUNT_ID.to_string(),
            arn: key_arn(&key_id),
            creation_date: created_at,
            current_key_material_id: key_material_id,
            description,
            enabled: true,
            encryption_algorithms: vec!["SYMMETRIC_DEFAULT".to_string()],
            key_id,
            key_manager: "CUSTOMER".to_string(),
            key_spec: "SYMMETRIC_DEFAULT".to_string(),
            key_state: "Enabled".to_string(),
            key_usage: "ENCRYPT_DECRYPT".to_string(),
            multi_region: false,
            origin: "AWS_KMS".to_string(),
        },
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

// ---------- CreateAlias ----------

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CreateAliasRequest {
    alias_name: String,
    target_key_id: String,
}

fn validate_alias_name(name: &str) -> Result<(), AppError> {
    let Some(rest) = name.strip_prefix("alias/") else {
        return Err(AppError::InvalidAliasName(
            "AliasName must begin with 'alias/'".into(),
        ));
    };
    if rest.is_empty() {
        return Err(AppError::InvalidAliasName("AliasName must not be empty".into()));
    }
    if rest.starts_with("aws/") {
        return Err(AppError::InvalidAliasName(
            "The alias/aws/ prefix is reserved for AWS managed keys".into(),
        ));
    }
    let valid_chars = rest
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '_' || c == '-');
    if !valid_chars {
        return Err(AppError::InvalidAliasName(
            "AliasName may only contain alphanumeric characters, '/', '_', and '-'".into(),
        ));
    }
    Ok(())
}

pub async fn create_alias(pool: &SqlitePool, body: &Bytes) -> Result<Response, AppError> {
    let req: CreateAliasRequest = parse_body(body)?;
    validate_alias_name(&req.alias_name)?;

    let target = keyid::resolve_key_only(pool, &req.target_key_id).await?;

    match db::insert_alias(pool, &req.alias_name, &target.key_id, now_epoch()).await {
        Ok(()) => {}
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            return Err(AppError::AlreadyExists(format!(
                "An alias with the name {} already exists",
                req.alias_name
            )));
        }
        Err(e) => return Err(e.into()),
    }

    Ok(StatusCode::OK.into_response())
}

// ---------- GenerateDataKey ----------

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct GenerateDataKeyRequest {
    key_id: String,
    key_spec: Option<String>,
    number_of_bytes: Option<u32>,
    encryption_context: Option<BTreeMap<String, String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct GenerateDataKeyResponse {
    ciphertext_blob: String,
    plaintext: String,
    key_id: String,
    key_material_id: String,
}

pub async fn generate_data_key(pool: &SqlitePool, body: &Bytes) -> Result<Response, AppError> {
    let req: GenerateDataKeyRequest = parse_body(body)?;

    let num_bytes: usize = match (req.key_spec.as_deref(), req.number_of_bytes) {
        (Some(_), Some(_)) => {
            return Err(AppError::Validation(
                "Specify either KeySpec or NumberOfBytes, not both".into(),
            ));
        }
        (None, None) => {
            return Err(AppError::Validation(
                "Either KeySpec or NumberOfBytes is required".into(),
            ));
        }
        (Some(spec), None) => match spec {
            "AES_256" => 32,
            "AES_128" => 16,
            other => {
                return Err(AppError::Validation(format!("Unsupported KeySpec '{other}'")));
            }
        },
        (None, Some(n)) => {
            if !(1..=1024).contains(&n) {
                return Err(AppError::Validation(
                    "NumberOfBytes must be between 1 and 1024".into(),
                ));
            }
            n as usize
        }
    };

    let key = keyid::resolve_any(pool, &req.key_id).await?;

    let plaintext = crypto::random_bytes(num_bytes);
    let aad = crypto::canonicalize_aad(&req.encryption_context.unwrap_or_default());
    let blob = crypto::encrypt(&key.key_material, &key.key_id, &plaintext, &aad);

    let response = GenerateDataKeyResponse {
        ciphertext_blob: crypto::base64_encode(&blob),
        plaintext: crypto::base64_encode(&plaintext),
        key_id: key_arn(&key.key_id),
        key_material_id: key.key_material_id,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}

// ---------- Decrypt ----------

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DecryptRequest {
    ciphertext_blob: String,
    key_id: Option<String>,
    encryption_context: Option<BTreeMap<String, String>>,
    #[allow(dead_code)]
    encryption_algorithm: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct DecryptResponse {
    plaintext: String,
    key_id: String,
    key_material_id: String,
    encryption_algorithm: String,
}

pub async fn decrypt(pool: &SqlitePool, body: &Bytes) -> Result<Response, AppError> {
    let req: DecryptRequest = parse_body(body)?;

    let blob_bytes = crypto::base64_decode(&req.ciphertext_blob)?;
    let decoded = crypto::decode_blob(&blob_bytes)?;

    let key = db::get_key_by_id(pool, &decoded.key_id)
        .await?
        .ok_or_else(|| {
            AppError::InvalidCiphertext("ciphertext refers to a key that no longer exists".into())
        })?;

    if let Some(requested) = &req.key_id {
        let requested_key = keyid::resolve_any(pool, requested).await?;
        if requested_key.key_id != key.key_id {
            return Err(AppError::IncorrectKey(
                "The specified KeyId does not match the key used to encrypt the ciphertext".into(),
            ));
        }
    }

    let aad = crypto::canonicalize_aad(&req.encryption_context.unwrap_or_default());
    let plaintext = crypto::decrypt(&key.key_material, &decoded.nonce, &decoded.ciphertext, &aad)?;

    let response = DecryptResponse {
        plaintext: crypto::base64_encode(&plaintext),
        key_id: key_arn(&key.key_id),
        key_material_id: key.key_material_id,
        encryption_algorithm: "SYMMETRIC_DEFAULT".to_string(),
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}
