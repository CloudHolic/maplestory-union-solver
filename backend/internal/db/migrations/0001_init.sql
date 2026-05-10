CREATE TABLE characters (
    nickname            TEXT    PRIMARY KEY,
    ocid                TEXT    NOT NULL,
    presets             TEXT,
    use_preset_no       INTEGER,
    union_fetched_at    INTEGER NOT NULL,
    last_selection      TEXT,
    last_selection_at   INTEGER,
    last_searched_at    INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_characters_last_searched ON characters(last_searched_at DESC);