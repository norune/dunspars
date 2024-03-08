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

CREATE TABLE change_move_value (
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