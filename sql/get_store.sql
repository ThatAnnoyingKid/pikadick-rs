SELECT 
    key_value 
FROM 
    kv_store 
WHERE 
    key_prefix = ? AND 
    key_name = ?
;