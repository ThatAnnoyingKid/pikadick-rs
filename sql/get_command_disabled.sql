SELECT 
    disabled 
FROM 
    disabled_commands 
WHERE 
    guild_id = ? AND name = ?;