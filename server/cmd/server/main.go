package main

import (
	"context"
	"errors"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/CloudHolic/maplestory-union-solver/server/internal/config"
	"github.com/CloudHolic/maplestory-union-solver/server/internal/db"
	"github.com/CloudHolic/maplestory-union-solver/server/internal/httpsrv"
	"github.com/jmoiron/sqlx"
	"github.com/labstack/echo/v5"
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

	e := httpsrv.New(httpsrv.Deps{
		Config: cfg,
		DB:     database,
	})

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	sc := echo.StartConfig{
		Address:         cfg.ServerAddr,
		HideBanner:      true,
		HidePort:        true,
		GracefulTimeout: 10 * time.Second,
	}

	slog.Info("server starting", "addr", cfg.ServerAddr)
	if err := sc.Start(ctx, e); err != nil && !errors.Is(err, http.ErrServerClosed) {
		slog.Error("server stopped with error", "err", err)
		os.Exit(1)
	}

	slog.Info("shutdown completed")
}

func setupLogger(logLevel, logFormat string) {
	level := slog.LevelInfo
	switch logLevel {
	case "debug":
		level = slog.LevelDebug
	case "info":
		level = slog.LevelInfo
	case "warn":
		level = slog.LevelWarn
	case "error":
		level = slog.LevelError
	}

	var handler slog.Handler
	if logFormat == "text" {
		handler = slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: level})
	} else {
		handler = slog.NewJSONHandler(os.Stderr, &slog.HandlerOptions{Level: level})
	}

	slog.SetDefault(slog.New(handler))
}
