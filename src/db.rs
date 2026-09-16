use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use sqlx::{Row, SqlitePool};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct Key {
    pub key_id: String,
    pub key_material: Vec<u8>,
    pub key_material_id: String,
    pub description: String,
    pub created_at: i64,
}

fn row_to_key(row: SqliteRow) -> Key {
    Key {
        key_id: row.get("key_id"),
        key_material: row.get("key_material"),
        key_material_id: row.get("key_material_id"),
        description: row.get("description"),
        created_at: row.get("created_at"),
    }
}

pub fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before the unix epoch")
        .as_secs() as i64
}

pub fn default_db_path() -> String {
    std::env::var("LOCAL_KMS_DB_PATH").unwrap_or_else(|_| "./.violet/kms-mock/persistence.db".to_string())
}

pub async fn connect_file(path: &str) -> Result<SqlitePool, sqlx::Error> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(sqlx::Error::Io)?;
        }
    }
    let options = SqliteConnectOptions::new().filename(path).create_if_missing(true);
    SqlitePoolOptions::new().connect_with(options).await
}

/// A single-connection in-memory pool, used by tests. Capping at one connection avoids the
/// footgun where a pool opens more than one connection to a bare `:memory:` URL and each
/// connection ends up with its own private, empty database.
pub async fn connect_in_memory() -> Result<SqlitePool, sqlx::Error> {
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub async fn insert_key(pool: &SqlitePool, key: &Key) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO keys (key_id, key_material, key_material_id, description, created_at) \
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&key.key_id)
    .bind(&key.key_material)
    .bind(&key.key_material_id)
    .bind(&key.description)
    .bind(key.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_key_by_id(pool: &SqlitePool, key_id: &str) -> Result<Option<Key>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT key_id, key_material, key_material_id, description, created_at \
         FROM keys WHERE key_id = ?",
    )
    .bind(key_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(row_to_key))
}

pub async fn get_key_by_alias(pool: &SqlitePool, alias_name: &str) -> Result<Option<Key>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT k.key_id, k.key_material, k.key_material_id, k.description, k.created_at \
         FROM aliases a JOIN keys k ON k.key_id = a.target_key_id \
         WHERE a.alias_name = ?",
    )
    .bind(alias_name)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(row_to_key))
}

pub async fn insert_alias(
    pool: &SqlitePool,
    alias_name: &str,
    target_key_id: &str,
    created_at: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO aliases (alias_name, target_key_id, created_at) VALUES (?, ?, ?)")
        .bind(alias_name)
        .bind(target_key_id)
        .bind(created_at)
        .execute(pool)
        .await?;
    Ok(())
}
