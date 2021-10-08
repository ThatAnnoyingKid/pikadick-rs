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

-- Temp until all ttt data is persisted
DROP TABLE IF EXISTS tic_tac_toe_games;
CREATE TABLE IF NOT EXISTS tic_tac_toe_games (
    id INTEGER NOT NULL UNIQUE PRIMARY KEY CHECK(TYPEOF(id) = 'integer'),
    game_state INTEGER NOT NULL CHECK(TYPEOF(game_state) = 'integer'),
    x_player TEXT NOT NULL CHECK(TYPEOF(x_player) = 'text'),
    o_player TEXT NOT NULL CHECK(TYPEOF(o_player) = 'text')
);

-- Draft of 2nd ttt table
/*
CREATE TABLE IF NOT EXISTS tic_tac_toe_player_info (
    guild_id INTEGER NULL CHECK(TYPEOF(guild_id) IN ('integer', 'null')),
    -- user_id 
    -- game_id INTEGER NOT NULL 
    -- Foregin key game_id -> tic_tac_toe_games.id
);
*/

COMMIT;