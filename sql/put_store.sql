INSERT OR REPLACE INTO kv_store (
    key_prefix, 
    key_name, 
    key_value
) VALUES (
    :prefix, 
    :key,
    JSONB(:value)
);