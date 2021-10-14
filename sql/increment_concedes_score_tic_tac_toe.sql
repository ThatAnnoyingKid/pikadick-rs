UPDATE 
    tic_tac_toe_scores 
SET 
    concedes = concedes + 1 
WHERE 
    guild_id = ? AND player = ?;