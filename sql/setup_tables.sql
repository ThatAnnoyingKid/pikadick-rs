PRAGMA page_size = 4096;
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = FULL;

BEGIN;

CREATE TABLE IF NOT EXISTS kv_store (
    key_prefix BLOB NULL CHECK(TYPEOF(key_prefix) IN ('blob', 'null')), 
    key_name BLOB NOT NULL CHECK(TYPEOF(key_name) = 'blob'),
    key_value BLOB NOT NULL CHECK(TYPEOF(key_value) = 'blob'),
    PRIMARY KEY(key_prefix, key_name),
    UNIQUE (key_prefix, key_name)
);

CREATE TABLE IF NOT EXISTS disabled_commands (
    guild_id INTEGER NOT NULL CHECK(TYPEOF(guild_id) = 'integer'),
    name TEXT NOT NULL CHECK(TYPEOF(name) = 'text'),
    disabled INTEGER NOT NULL CHECK(TYPEOF(disabled) = 'integer' AND disabled IN (0, 1)),
    PRIMARY KEY(guild_id, name),
    UNIQUE(guild_id, name)
);

CREATE TABLE IF NOT EXISTS reddit_embed_guild_settings (
    guild_id INTEGER NOT NULL PRIMARY KEY UNIQUE CHECK(TYPEOF(guild_id) = 'integer'),
    enabled INTEGER NOT NULL CHECK(TYPEOF(enabled) = 'integer' AND enabled IN (0, 1))
);

/*
CREATE TABLE IF NOT EXISTS tic_tac_toe_games (
    guild_id INTEGER NULL CHECK(TYPEOF(guild_id) IN ('integer', 'null')),
    -- user_id 
    -- game INTEGER NOT NULL 
);
*/

COMMIT;