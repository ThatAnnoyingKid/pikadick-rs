UPDATE 
    tic_tac_toe_scores 
SET 
    ties = ties + 1 
WHERE 
    guild_id = ? AND player IN (?, ?);