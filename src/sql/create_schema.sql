CREATE TABLE games (
    [id] INTEGER PRIMARY KEY,
    [name] TEXT NOT NULL,
    [order] INTEGER NOT NULL,
    [generation] INTEGER NOT NULL
);

CREATE TABLE evolutions (
    [id] INTEGER PRIMARY KEY,
    [evolution] TEXT NOT NULL
);

CREATE TABLE species (
    [id] INTEGER PRIMARY KEY,
    [name] TEXT NOT NULL,
    [type] TEXT NOT NULL,
    [evolution_id] INTEGER NOT NULL,
    FOREIGN KEY([evolution_id]) REFERENCES evolutions([id])
);

CREATE TABLE pokemon (
    [id] INTEGER PRIMARY KEY,
    [name] TEXT NOT NULL,
    [primary_type] TEXT NOT NULL,
    [secondary_type] TEXT,
    [attack] INTEGER NOT NULL,
    [defense] INTEGER NOT NULL,
    [special_attack] INTEGER NOT NULL,
    [special_defense] INTEGER NOT NULL,
    [speed] INTEGER NOT NULL,
    [species_id] INTEGER NOT NULL,
    FOREIGN KEY([species_id]) REFERENCES species([id])
);

CREATE TABLE pokemon_moves (
    [id] INTEGER PRIMARY KEY,
    [name] TEXT NOT NULL,
    [learn_method] TEXT NOT NULL,
    [learn_level] INTEGER NOT NULL,
    [generation] INTEGER NOT NULL,
    [pokemon_id] INTEGER NOT NULL,
    FOREIGN KEY([pokemon_id]) REFERENCES pokemon([id])
);

CREATE TABLE pokemon_abilities (
    [id] INTEGER PRIMARY KEY,
    [name] TEXT NOT NULL,
    [hidden] BOOLEAN NOT NULL,
    [pokemon_id] INTEGER NOT NULL,
    FOREIGN KEY([pokemon_id]) REFERENCES pokemon([id])
);

CREATE TABLE pokemon_type_changes (
    [id] INTEGER PRIMARY KEY,
    [primary_type] TEXT NOT NULL,
    [secondary_type] TEXT,
    [generation] INTEGER NOT NULL,
    [pokemon_id] INTEGER NOT NULL,
    FOREIGN KEY([pokemon_id]) REFERENCES pokemon([id])
);

CREATE TABLE moves (
    [id] INTEGER PRIMARY KEY,
    [name] TEXT NOT NULL,
    [power] INTEGER,
    [accuracy] INTEGER,
    [pp] INTEGER,
    [effect_chance] INTEGER,
    [effect] TEXT NOT NULL,
    [type] TEXT NOT NULL,
    [damage_class] TEXT NOT NULL,
    [generation] INTEGER NOT NULL
);

CREATE TABLE move_changes (
    [id] INTEGER PRIMARY KEY,
    [power] INTEGER,
    [accuracy] INTEGER,
    [pp] INTEGER,
    [effect_chance] INTEGER,
    [effect] TEXT,
    [type] TEXT,
    [generation] INTEGER NOT NULL,
    [move_id] INTEGER NOT NULL,
    FOREIGN KEY([move_id]) REFERENCES moves([id])
);

CREATE TABLE types (
    [id] INTEGER PRIMARY KEY,
    [name] TEXT NOT NULL,
    [no_damage_to] TEXT,
    [half_damage_to] TEXT,
    [double_damage_to] TEXT,
    [no_damage_from] TEXT,
    [half_damage_from] TEXT,
    [double_damage_from] TEXT,
    [generation] INTEGER NOT NULL
);

CREATE TABLE type_changes (
    [id] INTEGER PRIMARY KEY,
    [no_damage_to] TEXT,
    [half_damage_to] TEXT,
    [double_damage_to] TEXT,
    [no_damage_from] TEXT,
    [half_damage_from] TEXT,
    [double_damage_from] TEXT,
    [generation] INTEGER NOT NULL,
    [type_id] INTEGER NOT NULL,
    FOREIGN KEY([type_id]) REFERENCES types([id])
);

CREATE TABLE abilities (
    [id] INTEGER PRIMARY KEY,
    [name] TEXT NOT NULL,
    [effect] TEXT NOT NULL,
    [generation] ITNEGER NOT NULL
);