SELECT 
    (wins - (losses + concedes)) as score, 
    player, 
    wins, 
    losses, 
    ties, 
    concedes 
FROM 
    tic_tac_toe_scores 
WHERE 
    guild_id = ? 
ORDER BY 
    score DESC
LIMIT 
    10;