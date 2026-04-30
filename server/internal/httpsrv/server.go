// Package httpsrv assembles the Echo instance.
package httpsrv

import (
	"time"

	"github.com/CloudHolic/maplestory-union-solver/server/internal/characters"
	"github.com/CloudHolic/maplestory-union-solver/server/internal/config"
	"github.com/CloudHolic/maplestory-union-solver/server/internal/nexon"

	"github.com/jmoiron/sqlx"
	"github.com/labstack/echo/v5"
	"github.com/labstack/echo/v5/middleware"
)

// Deps gathers external dependencies the HTTP server needs to construct its handlers.
type Deps struct {
	Config *config.Config
	DB     *sqlx.DB
}

// New returns a fully wired Echo instance with middleware registered and routes method.
func New(deps Deps) *echo.Echo {
	e := echo.New()

	e.IPExtractor = echo.ExtractIPDirect()

	e.Use(middleware.Recover())
	e.Use(middleware.RequestLogger())
	e.Use(middleware.BodyLimit(1 << 20))
	e.Use(middleware.CORSWithConfig(middleware.CORSConfig{
		AllowOrigins: []string{"http://localhost:5173"},
		AllowMethods: []string{"GET"},
	}))

	repo := characters.NewRepository(deps.DB)
	nx := nexon.NewClient(deps.Config.NexonAPIKey)

	const unionTTL = 60 * time.Second
	svc := characters.NewService(repo, nx, unionTTL)
	chHandler := characters.NewHandler(svc)

	api := e.Group("/api")
	api.GET("/characters/:nickname", chHandler.GetByNickname)

	return e
}
