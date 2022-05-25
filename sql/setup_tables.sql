PRAGMA page_size = 4096;
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = FULL;

CREATE TABLE IF NOT EXISTS kv_store (
    key_prefix BLOB NULL CHECK(TYPEOF(key_prefix) IN ('blob', 'null')), 
    key_name BLOB NOT NULL CHECK(TYPEOF(key_name) = 'blob'),
    key_value BLOB NOT NULL CHECK(TYPEOF(key_value) = 'blob'),
    PRIMARY KEY (key_prefix, key_name),
    UNIQUE (key_prefix, key_name)
);

CREATE TABLE IF NOT EXISTS disabled_commands (
    guild_id INTEGER NOT NULL CHECK(TYPEOF(guild_id) = 'integer'),
    name TEXT NOT NULL CHECK(TYPEOF(name) = 'text'),
    disabled INTEGER NOT NULL CHECK(TYPEOF(disabled) = 'integer' AND disabled IN (0, 1)),
    PRIMARY KEY (guild_id, name),
    UNIQUE (guild_id, name)
);

CREATE TABLE IF NOT EXISTS reddit_embed_guild_settings (
    guild_id INTEGER NOT NULL PRIMARY KEY UNIQUE,
    enabled INTEGER NOT NULL CHECK(enabled IN (0, 1))
) STRICT;

CREATE TABLE IF NOT EXISTS tic_tac_toe_games (
    id INTEGER PRIMARY KEY UNIQUE NOT NULL CHECK(TYPEOF(id) = 'integer'),
    board INTEGER NOT NULL CHECK(TYPEOF(board) = 'integer'),
    x_player INTEGER NULL CHECK(TYPEOF(x_player) IN ('integer', 'null')),
    o_player INTEGER NULL CHECK(TYPEOF(o_player) IN ('integer', 'null')),
    guild_id TEXT NOT NULL CHECK(TYPEOF(guild_id) = 'text'),
    UNIQUE (guild_id, x_player, o_player),
    UNIQUE (guild_id, x_player),
    UNIQUE (guild_id, o_player)
);

CREATE TABLE IF NOT EXISTS tic_tac_toe_scores (
    guild_id TEXT NOT NULL CHECK(TYPEOF(guild_id) = 'text'),
    player INTEGER NOT NULL CHECK(TYPEOF(player) = 'integer'),
    wins INTEGER NOT NULL DEFAULT 0 CHECK(TYPEOF(wins) = 'integer'),
    losses INTEGER NOT NULL DEFAULT 0 CHECK(TYPEOF(losses) = 'integer'),
    concedes INTEGER NOT NULL DEFAULT 0 CHECK(TYPEOF(concedes) = 'integer'),
    ties INTEGER NOT NULL DEFAULT 0 CHECK(TYPEOF(ties) = 'integer'),
    PRIMARY KEY (guild_id, player),
    UNIQUE (guild_id, player)
);

CREATE TABLE IF NOT EXISTS tiktok_embed_guild_settings (
    guild_id INTEGER NOT NULL PRIMARY KEY UNIQUE,
    
    -- flags for tiktok embed settings
    --
    -- bit | name         | Description
    -- 0   | enabled?     | Whether the bot should try to embed links
    -- 1   | delete-link? | Whether the bot should delete the original link on success
    flags INTEGER NOT NULL DEFAULT 0
) STRICT;