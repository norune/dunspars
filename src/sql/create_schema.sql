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
    [pp] INTEGER
);