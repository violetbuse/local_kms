use sqlx::SqlitePool;

use crate::db::{self, Key};
use crate::error::AppError;

pub const FAKE_REGION: &str = "us-east-2";
pub const FAKE_ACCOUNT_ID: &str = "111122223333";

pub fn key_arn(key_id: &str) -> String {
    format!("arn:aws:kms:{FAKE_REGION}:{FAKE_ACCOUNT_ID}:key/{key_id}")
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParsedKeyId {
    Raw(String),
    KeyArn(String),
    Alias(String),
    AliasArn(String),
    Malformed,
}

/// Classifies a `KeyId`/`TargetKeyId` string per the four forms AWS KMS accepts: a bare key
/// id, a key ARN, an `alias/Name`, or an alias ARN. Region/account segments inside a supplied
/// ARN are not validated — only the `key/<id>` or `alias/<name>` resource suffix is used, so
/// the mock stays permissive about whatever fake ARNs a caller's fixtures use.
pub fn parse_key_identifier(s: &str) -> ParsedKeyId {
    if s.starts_with("alias/") {
        return ParsedKeyId::Alias(s.to_string());
    }
    if s.starts_with("arn:aws:kms:") {
        if let Some(idx) = s.find(":key/") {
            return ParsedKeyId::KeyArn(s[idx + ":key/".len()..].to_string());
        }
        if let Some(idx) = s.find(":alias/") {
            // Keep the "alias/Name" form (skip the leading colon only).
            return ParsedKeyId::AliasArn(s[idx + 1..].to_string());
        }
        return ParsedKeyId::Malformed;
    }
    ParsedKeyId::Raw(s.to_string())
}

/// Resolves a `KeyId` given in any of the four accepted forms. Used by `GenerateDataKey` and
/// `Decrypt`, both of which accept a key identified directly or via one of its aliases.
pub async fn resolve_any(pool: &SqlitePool, key_id: &str) -> Result<Key, AppError> {
    match parse_key_identifier(key_id) {
        ParsedKeyId::Raw(id) | ParsedKeyId::KeyArn(id) => db::get_key_by_id(pool, &id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Key '{key_id}' does not exist"))),
        ParsedKeyId::Alias(name) | ParsedKeyId::AliasArn(name) => db::get_key_by_alias(pool, &name)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Alias '{key_id}' does not exist"))),
        ParsedKeyId::Malformed => Err(AppError::NotFound(format!("Key '{key_id}' does not exist"))),
    }
}

/// Resolves a `KeyId` that must refer to a KMS key directly, not an alias. Used for
/// `CreateAlias`'s `TargetKeyId`, which AWS documents as requiring a key id/ARN.
pub async fn resolve_key_only(pool: &SqlitePool, key_id: &str) -> Result<Key, AppError> {
    match parse_key_identifier(key_id) {
        ParsedKeyId::Raw(id) | ParsedKeyId::KeyArn(id) => db::get_key_by_id(pool, &id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Key '{key_id}' does not exist"))),
        _ => Err(AppError::NotFound(format!("Key '{key_id}' does not exist"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_raw_id() {
        assert_eq!(
            parse_key_identifier("1234abcd-12ab-34cd-56ef-1234567890ab"),
            ParsedKeyId::Raw("1234abcd-12ab-34cd-56ef-1234567890ab".to_string())
        );
    }

    #[test]
    fn classifies_key_arn() {
        assert_eq!(
            parse_key_identifier("arn:aws:kms:us-east-2:111122223333:key/1234abcd"),
            ParsedKeyId::KeyArn("1234abcd".to_string())
        );
    }

    #[test]
    fn classifies_alias() {
        assert_eq!(
            parse_key_identifier("alias/ExampleAlias"),
            ParsedKeyId::Alias("alias/ExampleAlias".to_string())
        );
    }

    #[test]
    fn classifies_alias_arn() {
        assert_eq!(
            parse_key_identifier("arn:aws:kms:us-east-2:111122223333:alias/ExampleAlias"),
            ParsedKeyId::AliasArn("alias/ExampleAlias".to_string())
        );
    }

    #[test]
    fn classifies_malformed_arn() {
        assert_eq!(
            parse_key_identifier("arn:aws:kms:us-east-2:111122223333:bogus/Foo"),
            ParsedKeyId::Malformed
        );
    }
}
