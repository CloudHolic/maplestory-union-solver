package db

import (
	"fmt"

	"github.com/jmoiron/sqlx"
	_ "modernc.org/sqlite"
)

func Open(dsn string) (*sqlx.DB, error) {
	db, err := sqlx.Connect("sqlite", dsn)
	if err != nil {
		return nil, fmt.Errorf("opening sqlite: %w", err)
	}

	db.SetMaxOpenConns(1)

	if err := applyMigrations(db); err != nil {
		_ = db.Close()
		return nil, fmt.Errorf("applying migrations: %w", err)
	}

	return db, nil
}
