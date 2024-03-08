CREATE TABLE games (
    [id] INTEGER PRIMARY KEY,
    [name] TEXT NOT NULL,
    [order] INTEGER NOT NULL,
    [generation] INTEGER NOT NULL
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
    [generation] u8 NOT NULL
);

CREATE TABLE type_changes (
    [id] INTEGER PRIMARY KEY,
    [no_damage_to] TEXT,
    [half_damage_to] TEXT,
    [double_damage_to] TEXT,
    [no_damage_from] TEXT,
    [half_damage_from] TEXT,
    [double_damage_from] TEXT,
    [generation] u8 NOT NULL,
    [type_id] INTEGER NOT NULL,
    FOREIGN KEY([type_id]) REFERENCES types([id])
);
