UPDATE 
    tic_tac_toe_scores 
SET 
    wins = wins + 1 
WHERE 
    guild_id = ? AND player = ?;