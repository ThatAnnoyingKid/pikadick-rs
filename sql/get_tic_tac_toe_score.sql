SELECT 
    wins, 
    losses, 
    ties, 
    concedes 
FROM 
    tic_tac_toe_scores 
WHERE 
    guild_id = ? AND player = ?;