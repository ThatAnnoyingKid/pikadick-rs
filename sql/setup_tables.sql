PRAGMA page_size = 4096;
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;
PRAGMA synchronous = FULL;

CREATE TABLE IF NOT EXISTS kv_store (
    key BLOB PRIMARY KEY UNIQUE NOT NULL CHECK(TYPEOF(key) = 'blob'),
    value BLOB NOT NULL CHECK(TYPEOF(value) = 'blob')
);

CREATE TABLE IF NOT EXISTS guilds (
    id INTEGER PRIMARY KEY UNIQUE NOT NULL CHECK(TYPEOF(id) = 'integer')
);

CREATE TABLE IF NOT EXISTS disabled_commands (
    guild_id INTEGER NOT NULL CHECK(TYPEOF(guild_id) = 'integer'),
    name TEXT NOT NULL CHECK(TYPEOF(name) = 'text'),
    disabled INTEGER NOT NULL CHECK(TYPEOF(disabled) = 'integer' AND disabled IN (0, 1)),
    FOREIGN KEY(guild_id) REFERENCES guilds(id),
    PRIMARY KEY(guild_id, name),
    UNIQUE(guild_id, name)
);