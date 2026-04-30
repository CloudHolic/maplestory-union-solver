package characters

import (
	"context"
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	"time"

	"github.com/CloudHolic/maplestory-union-solver/server/internal/nexon"
	"github.com/jmoiron/sqlx"
)

// ErrNotFound is returned when no row exists for the given nickname.
var ErrNotFound = errors.New("characters: not in cache")

// CacheEntry is the persisted form of a cached character.
type CacheEntry struct {
	Nickname        string         `db:"nickname"`
	OCID            string         `db:"ocid"`
	Presets         sql.NullString `db:"presets"`
	UsePresetNo     sql.NullInt64  `db:"use_preset_no"`
	UnionFetchedAt  sql.NullInt64  `db:"union_fetched_at"`
	LastSelection   sql.NullString `db:"last_selection"`
	LastSelectionAt sql.NullInt64  `db:"last_selection_at"`
	LastSearchedAt  int64          `db:"last_searched_at"`
}

// IsUnionFresh reports whether the cached union data is still within the TTL window.
func (e *CacheEntry) IsUnionFresh(now time.Time, ttl time.Duration) bool {
	if !e.UnionFetchedAt.Valid {
		return false
	}

	age := now.Unix() - e.UnionFetchedAt.Int64
	return age < int64(ttl.Seconds())
}

// DecodePresets parses the stored JSON into the domain type.
func (e *CacheEntry) DecodePresets() ([][]nexon.Block, error) {
	if !e.Presets.Valid || e.Presets.String == "" {
		return nil, nil
	}

	var presets [][]nexon.Block
	if err := json.Unmarshal([]byte(e.Presets.String), &presets); err != nil {
		return nil, fmt.Errorf("decoding presets: %w", err)
	}

	return presets, nil
}

// Repository is the persistence boundary for the characters cache.
type Repository struct {
	db *sqlx.DB
}

// NewRepository constructs a Repository bound to the given database.
func NewRepository(db *sqlx.DB) *Repository {
	return &Repository{db: db}
}

// Get returns the cache entry for the nickname, or ErrNotFound when no row exists.
func (r *Repository) Get(ctx context.Context, nickname string) (*CacheEntry, error) {
	var entry CacheEntry
	err := r.db.GetContext(ctx, &entry,
		`SELECT * FROM characters WHERE nickname = ?`, nickname)

	if errors.Is(err, sql.ErrNoRows) {
		return nil, ErrNotFound
	}

	if err != nil {
		return nil, fmt.Errorf("get %q: %w", nickname, err)
	}

	return &entry, nil
}

// Upsert inserts a new row when the nickname has never been searched.
func (r *Repository) Upsert(
	ctx context.Context,
	nickname, ocid string,
	union *nexon.UnionData,
	now time.Time,
) error {
	presetsJSON, err := json.Marshal(union.Presets)
	if err != nil {
		return fmt.Errorf("encoding presets: %w", err)
	}

	nowSec := now.Unix()
	_, err = r.db.ExecContext(ctx, `
		INSERT INTO characters 
		    (nickname, ocid, presets, use_preset_no, union_fetched_at, last_searched_at)
		VALUES (?, ?, ?, ?, ?, ?)
		ON CONFLICT (nickname) DO UPDATE SET
			 ocid				= excluded.ocid,
			 presets			= excluded.presets,
			 use_preset_no		= excluded.use_preset_no,
			 union_fetched_at	= excluded.union_fetched_at,
			 last_searched_at	= excluded.last_searched_at
	`, nickname, ocid, string(presetsJSON), union.UsePresetNo, nowSec, nowSec)

	if err != nil {
		return fmt.Errorf("upsert %q: %w", nickname, err)
	}

	return nil
}

// RefreshUnion updates union-related columns and bumps last_searched_at.
func (r *Repository) RefreshUnion(
	ctx context.Context,
	nickname string,
	union *nexon.UnionData,
	now time.Time,
) error {
	presetsJSON, err := json.Marshal(union.Presets)
	if err != nil {
		return fmt.Errorf("encoding presets: %w", err)
	}

	nowSec := now.Unix()
	_, err = r.db.ExecContext(ctx, `
		UPDATE characters SET
			presets				= ?,
			use_preset_no		= ?,
			union_fetched_at	= ?,
			last_searched_at	= ?
		WHERE nickname = ?
	`, string(presetsJSON), union.UsePresetNo, nowSec, nowSec, nickname)

	if err != nil {
		return fmt.Errorf("refresh union for %q: %w", nickname, err)
	}

	return nil
}

// TouchSearchedAt updates only last_searched_at. Used on cache hits.
func (r *Repository) TouchSearchedAt(
	ctx context.Context,
	nickname string,
	now time.Time,
) error {
	_, err := r.db.ExecContext(ctx,
		` UPDATE characters SET last_searched_at = ? WHERE nickname = ?`,
		now.Unix(), nickname)

	if err != nil {
		return fmt.Errorf("touch %q: %w", nickname, err)
	}

	return nil
}
