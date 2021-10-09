SELECT 
    id,
    board, 
    x_player, 
    o_player
FROM 
    tic_tac_toe_games 
WHERE
    guild_id = :guild_id AND 
    (x_player = :user_id OR o_player = :user_id);