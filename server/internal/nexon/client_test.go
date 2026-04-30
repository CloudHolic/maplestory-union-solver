package nexon

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"
)

func TestGetOCID_Success(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if got := r.Header.Get("x-nxopen-api-key"); got != "test-key" {
			t.Errorf("api key header missing or wrong: got %q", got)
		}
		if got := r.URL.Query().Get("character_name"); got != "구름두컵" {
			t.Errorf("character_name query: got %q want 구름두컵", got)
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"ocid":"abc123"}`))
	}))
	defer srv.Close()

	c := NewClient("test-key")
	c.baseURL = srv.URL

	ocid, err := c.GetOCID(context.Background(), "구름두컵")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if ocid != "abc123" {
		t.Errorf("ocid: got %q want abc123", ocid)
	}
}

func TestGetOCID_CharacterNotFound(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		_, _ = w.Write([]byte(`{"error":{"name":"OPENAPI00004","message":"Please input valid parameter"}}`))
	}))
	defer srv.Close()

	c := NewClient("test-key")
	c.baseURL = srv.URL

	_, err := c.GetOCID(context.Background(), "no-such-character")
	if !errors.Is(err, ErrCharacterNotFound) {
		t.Fatalf("want ErrCharacterNotFound, got %v", err)
	}
}

func TestGetOCID_RateLimited(t *testing.T) {
	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusTooManyRequests)
		_, _ = w.Write([]byte(`{"error":{"name":"OPENAPI00007","message":"too many requests"}}`))
	}))
	defer srv.Close()

	c := NewClient("test-key")
	c.baseURL = srv.URL

	_, err := c.GetOCID(context.Background(), "x")
	if !errors.Is(err, ErrRateLimited) {
		t.Fatalf("want ErrRateLimited, got %v", err)
	}
}

func TestGetUnionData_FromFixture(t *testing.T) {
	fixturePath := filepath.Join("testdata", "example_union.json")
	data, err := os.ReadFile(fixturePath)
	if err != nil {
		t.Skipf("fixture not present at %s; skipping", fixturePath)
	}

	srv := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write(data)
	}))
	defer srv.Close()

	c := NewClient("test-key")
	c.baseURL = srv.URL

	got, err := c.GetUnionData(context.Background(), "test-ocid")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(got.Presets) != PresetCount {
		t.Errorf("presets length: got %d want %d", len(got.Presets), PresetCount)
	}

	for i, preset := range got.Presets {
		for j, b := range preset {
			if b.Level <= 0 {
				t.Errorf("preset[%d] block[%d] level non-positive: %d", i, j, b.Level)
			}
			if b.Type == "" {
				t.Errorf("preset[%d] block[%d] empty type", i, j)
			}
		}
	}

	out, err := json.Marshal(got)
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	var roundTripped UnionData
	if err := json.Unmarshal(out, &roundTripped); err != nil {
		t.Fatalf("round-trip unmarshal: %v", err)
	}
	if len(roundTripped.Presets) != PresetCount {
		t.Errorf("round-trip preset count: got %d", len(roundTripped.Presets))
	}
}
