SELECT 
    JSON(kv_store.key_value) AS kv_store
FROM 
    kv_store 
WHERE 
    key_prefix = :prefix AND 
    key_name = :key;