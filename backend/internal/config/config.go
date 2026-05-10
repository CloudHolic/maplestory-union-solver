// Package config loads runtime configuration from the environment,
// with optional .env file support for local development.
package config

import (
	"errors"
	"os"
	"strconv"
	"strings"

	"github.com/joho/godotenv"
)

// Config holds all runtime settings sourced from environment variables.
type Config struct {
	NexonAPIKey    string
	LogLevel       string
	LogFormat      string
	ServerAddr     string
	DatabaseURL    string
	TrustedProxies []string
	RateLimit      int
}

// Load reads configuration from environment variables.
// Returns an error if any required setting is missing.
func Load() (*Config, error) {
	_ = godotenv.Load()

	cfg := &Config{
		NexonAPIKey:    os.Getenv("NEXON_API_KEY"),
		LogLevel:       os.Getenv("LOG_LEVEL"),
		LogFormat:      os.Getenv("LOG_INFO"),
		ServerAddr:     getEnvDefault("SERVER_ADDR", ":8080"),
		DatabaseURL:    getEnvDefault("DATABASE_URL", "file:./data/union.db?_pragma=journal_mode(WAL)&_pragma=busy_timeout(5000"),
		TrustedProxies: parseCSV(os.Getenv("TRUSTED_PROXIES")),
		RateLimit:      getEnvInt("RATE_LIMIT", 30),
	}

	if cfg.NexonAPIKey == "" {
		return nil, errors.New("NEXON_API_KEY is required")
	}

	return cfg, nil
}

func getEnvDefault(key, def string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}

	return def
}

func getEnvInt(key string, def int) int {
	v := os.Getenv(key)
	if v == "" {
		return def
	}

	n, err := strconv.Atoi(v)
	if err != nil || n <= 0 {
		return def
	}

	return n
}

func parseCSV(s string) []string {
	if s == "" {
		return nil
	}

	parts := strings.Split(s, ",")
	out := make([]string, 0, len(parts))

	for _, p := range parts {
		if p = strings.TrimSpace(p); p != "" {
			out = append(out, p)
		}
	}

	return out
}
