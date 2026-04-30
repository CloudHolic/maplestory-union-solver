CREATE TABLE characters (
    nickname            TEXT    PRIMARY KEY,
    nexon_data          TEXT    NOT NULL,   -- NEXON Response JSON
    nexon_fetched_at    INTEGER NOT NULL,   -- UNIX Epoch (sec). For TTL
    last_selection      TEXT,               -- JSON
    last_selection_at   INTEGER,
    last_searched_at    INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_characters_last_searched ON characters(last_searched_at DESC);