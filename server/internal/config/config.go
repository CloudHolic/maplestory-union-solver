package config

import (
	"errors"
	"os"
	"strings"

	"github.com/joho/godotenv"
)

type Config struct {
	NexonAPIKey    string
	LogLevel       string
	LogFormat      string
	ServerAddr     string
	DatabaseURL    string
	TrustedProxies []string
}

func Load() (*Config, error) {
	_ = godotenv.Load()

	cfg := &Config{
		NexonAPIKey:    os.Getenv("NEXON_API_KEY"),
		LogLevel:       os.Getenv("LOG_LEVEL"),
		LogFormat:      os.Getenv("LOG_INFO"),
		ServerAddr:     getEnvDefault("SERVER_ADDR", ":8080"),
		DatabaseURL:    getEnvDefault("DATABASE_URL", "file:./data/union.db?_pragma=journal_mode(WAL)&_pragma=busy_timeout(5000"),
		TrustedProxies: parseCSV(os.Getenv("TRUSTED_PROXIES")),
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
