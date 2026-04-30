// Package db owns the SQLite connection lifecycle and applies embedded
// schema migrations on startup.
package db

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/jmoiron/sqlx"
	_ "modernc.org/sqlite"
)

// Open opens a SQLite database at the given DSN, ensures the parent directory exists,
// applies all embedded migrations, and returns a ready-to-use *sqlx.DB.
// The caller is responsible for closing it.
func Open(dsn string) (*sqlx.DB, error) {
	if path := extractFilePath(dsn); path != "" {
		if dir := filepath.Dir(path); dir != "" && dir != "." {
			if err := os.MkdirAll(dir, 0750); err != nil {
				return nil, fmt.Errorf("creating db directory %q: %w", dir, err)
			}
		}
	}

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

func extractFilePath(dsn string) string {
	s := strings.TrimPrefix(dsn, "file:")
	if i := strings.IndexByte(s, '?'); i >= 0 {
		s = s[:i]
	}

	if s == "" || s == ":memory:" {
		return ""
	}

	return s
}
