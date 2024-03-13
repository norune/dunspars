SELECT
    m.[name], p.[learn_method], p.[learn_level]
FROM pokemon_moves AS p
JOIN moves AS m
    ON m.[id] = p.[move_id]
WHERE p.[pokemon_id] = ?1 
    AND p.[generation] = ?2;