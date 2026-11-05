PRAGMA page_size = 4096;
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = FULL;

CREATE TABLE IF NOT EXISTS kv_store (
    key_prefix BLOB NOT NULL, 
    key_name BLOB NOT NULL,
    key_value BLOB NOT NULL,
    PRIMARY KEY (key_prefix, key_name)
) STRICT;

CREATE TABLE IF NOT EXISTS tic_tac_toe_games (
    id INTEGER PRIMARY KEY UNIQUE NOT NULL,
    board INTEGER NOT NULL,
    x_player INTEGER NULL,
    o_player INTEGER NULL,
    guild_id TEXT NOT NULL,
    UNIQUE (guild_id, x_player, o_player),
    UNIQUE (guild_id, x_player),
    UNIQUE (guild_id, o_player)
) STRICT;

CREATE TABLE IF NOT EXISTS tic_tac_toe_scores (
    guild_id TEXT NOT NULL,
    player INTEGER NOT NULL,
    wins INTEGER NOT NULL DEFAULT 0,
    losses INTEGER NOT NULL DEFAULT 0,
    concedes INTEGER NOT NULL DEFAULT 0,
    ties INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (guild_id, player),
    UNIQUE (guild_id, player)
) STRICT;

CREATE TABLE IF NOT EXISTS tiktok_embed_guild_settings (
    guild_id INTEGER NOT NULL PRIMARY KEY UNIQUE,
    
    -- flags for tiktok embed settings
    --
    -- bit | name         | Description
    -- 0   | enabled?     | Whether the bot should try to embed links
    -- 1   | delete-link? | Whether the bot should delete the original link on success
    flags INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE IF NOT EXISTS rule34_post (
    id INTEGER NOT NULL PRIMARY KEY,
    tags TEXT NOT NULL,
    file_url TEXT NOT NULL,
    last_fetched INTEGER NOT NULL
) STRICT;

CREATE TABLE IF NOT EXISTS rule34_tag (
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
) STRICT;

CREATE TABLE IF NOT EXISTS rule34_post_tag (
    post_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    
    PRIMARY KEY (post_id, tag_id),
    FOREIGN KEY (post_id) REFERENCES rule34_post (id),
    FOREIGN KEY (tag_id) REFERENCES rule34_tag (id)
) STRICT;

CREATE TABLE IF NOT EXISTS rule34_query_stat (
    query TEXT NOT NULL PRIMARY KEY,
    queries_since_last_fetch INTEGER NOT NULL,
    last_fetched INTEGER NOT NULL
) STRICT;