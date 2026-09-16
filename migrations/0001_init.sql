CREATE TABLE keys (
    key_id          TEXT PRIMARY KEY,
    key_material    BLOB NOT NULL,
    key_material_id TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    created_at      INTEGER NOT NULL
);

CREATE TABLE aliases (
    alias_name    TEXT PRIMARY KEY,
    target_key_id TEXT NOT NULL REFERENCES keys(key_id),
    created_at    INTEGER NOT NULL
);
