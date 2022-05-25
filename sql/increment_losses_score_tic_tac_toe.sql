UPDATE 
    tic_tac_toe_scores 
SET 
    losses = losses + 1 
WHERE 
    guild_id = ? AND player = ?;