SELECT 
    a.[name], p.[is_hidden]
FROM pokemon_abilities AS p
JOIN abilities AS a
    ON a.[id] = p.[ability_id]
WHERE pokemon_id = ?1;