SELECT 
    x_player,
    o_player
FROM 
    tic_tac_toe_games 
WHERE 
    guild_id = :guild_id AND 
    (
        x_player IN (:author, :opponent) OR 
        o_player IN (:author, :opponent)
    );