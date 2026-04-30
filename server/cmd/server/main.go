package main

import (
	"log/slog"
	"os"

	"github.com/CloudHolic/maplestory-union-solver/server/internal/config"
	"github.com/CloudHolic/maplestory-union-solver/server/internal/db"
	"github.com/jmoiron/sqlx"
)

func main() {
	cfg, err := config.Load()
	if err != nil {
		slog.Error("config load failed", "err", err)
		os.Exit(1)
	}

	setupLogger(cfg.LogLevel, cfg.LogFormat)

	database, err := db.Open(cfg.DatabaseURL)
	if err != nil {
		slog.Error("db open failed", "err", err)
		os.Exit(1)
	}
	defer func(database *sqlx.DB) {
		_ = database.Close()
	}(database)

	slog.Info("server skeleton ready",
		"addr", cfg.ServerAddr,
		"db", cfg.DatabaseURL)
}

func setupLogger(logLevel, logFormat string) {
	level := slog.LevelInfo
	switch logLevel {
	case "debug":
		level = slog.LevelDebug
		break
	case "info":
		level = slog.LevelInfo
		break
	case "warn":
		level = slog.LevelWarn
		break
	case "error":
		level = slog.LevelError
		break
	}

	var handler slog.Handler
	if logFormat == "text" {
		handler = slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: level})
	} else {
		handler = slog.NewJSONHandler(os.Stderr, &slog.HandlerOptions{Level: level})
	}

	slog.SetDefault(slog.New(handler))
}
