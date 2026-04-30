package characters

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"time"

	"github.com/CloudHolic/maplestory-union-solver/server/internal/nexon"
)

type nexonClient interface {
	GetOCID(ctx context.Context, nickname string) (string, error)
	GetUnionData(ctx context.Context, ocid string) (*nexon.UnionData, error)
}

// Clock returns the current time.
type Clock func() time.Time

// CharacterView is the response shape the handler renders to JSON.
type CharacterView struct {
	Nickname       string          `json:"nickname"`
	OCID           string          `json:"ocid"`
	Presets        [][]nexon.Block `json:"presets"`
	UsePresetNo    int             `json:"usePresetNo"`
	LastSelection  *string         `json:"lastSelection"`
	LastSearchedAt int64           `json:"lastSearchedAt"`
}

// Service orchestrates cache lookup, NEXON fetch on miss/stale, and persistence.
type Service struct {
	repo     *Repository
	nexon    nexonClient
	unionTTL time.Duration
	now      Clock
}

// NewService wires the dependencies.
// unionTTL controls how long a fetched union payload is treated as fresh;
// OCID is permanent.
func NewService(repo *Repository, nx nexonClient, unionTTL time.Duration) *Service {
	return &Service{
		repo:     repo,
		nexon:    nx,
		unionTTL: unionTTL,
		now:      time.Now,
	}
}

// GetByNickname returns a populated CharacterView, fetching from NEXON only when necessary.
func (s *Service) GetByNickname(ctx context.Context, nickname string) (*CharacterView, error) {
	now := s.now()
	entry, err := s.repo.Get(ctx, nickname)

	switch {
	case errors.Is(err, ErrNotFound):
		return s.firstFetch(ctx, nickname, now)
	case err != nil:
		return nil, err
	case !entry.IsUnionFresh(now, s.unionTTL):
		return s.refreshFromUpstream(ctx, entry, now)
	default:
		return s.serveFromCache(ctx, entry, now)
	}
}

func (s *Service) firstFetch(ctx context.Context, nickname string, now time.Time) (*CharacterView, error) {
	ocid, err := s.nexon.GetOCID(ctx, nickname)
	if err != nil {
		return nil, err
	}

	union, err := s.nexon.GetUnionData(ctx, ocid)
	if err != nil {
		return nil, err
	}

	if err := s.repo.Upsert(ctx, nickname, ocid, union, now); err != nil {
		return nil, fmt.Errorf("persisting first fetch: %w", err)
	}

	return &CharacterView{
		Nickname:       nickname,
		OCID:           ocid,
		Presets:        union.Presets,
		UsePresetNo:    union.UsePresetNo,
		LastSelection:  nil,
		LastSearchedAt: now.Unix(),
	}, nil
}

func (s *Service) refreshFromUpstream(ctx context.Context, entry *CacheEntry, now time.Time) (*CharacterView, error) {
	union, err := s.nexon.GetUnionData(ctx, entry.OCID)
	if err != nil {
		return nil, err
	}

	if err := s.repo.RefreshUnion(ctx, entry.Nickname, union, now); err != nil {
		return nil, fmt.Errorf("persisting refresh: %w", err)
	}

	return &CharacterView{
		Nickname:       entry.Nickname,
		OCID:           entry.OCID,
		Presets:        union.Presets,
		UsePresetNo:    union.UsePresetNo,
		LastSelection:  nullStringToPtr(entry.LastSelection),
		LastSearchedAt: now.Unix(),
	}, nil
}

func (s *Service) serveFromCache(ctx context.Context, entry *CacheEntry, now time.Time) (*CharacterView, error) {
	presets, err := entry.DecodePresets()
	if err != nil {
		return nil, fmt.Errorf("decoding cached presets: %w", err)
	}

	if err := s.repo.TouchSearchedAt(ctx, entry.Nickname, now); err != nil {
		return nil, fmt.Errorf("touching last_searched_at: %w", err)
	}

	return &CharacterView{
		Nickname:       entry.Nickname,
		OCID:           entry.OCID,
		Presets:        presets,
		UsePresetNo:    int(entry.UsePresetNo.Int64),
		LastSelection:  nullStringToPtr(entry.LastSelection),
		LastSearchedAt: now.Unix(),
	}, nil
}

func nullStringToPtr(ns sql.NullString) *string {
	if !ns.Valid {
		return nil
	}

	v := ns.String
	return &v
}
