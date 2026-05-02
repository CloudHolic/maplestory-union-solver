// Package httpsrv assembles the Echo instance.
package httpsrv

import (
	"net/http"
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
	e.Use(middleware.BodyLimit(1 << 20)) // 1MB
	e.Use(middleware.CORSWithConfig(middleware.CORSConfig{
		AllowOrigins: []string{"http://localhost:5173"},
		AllowMethods: []string{"GET", "PUT"},
	}))

	repo := characters.NewRepository(deps.DB)
	nx := nexon.NewClient(deps.Config.NexonAPIKey)

	const unionTTL = 60 * time.Second
	svc := characters.NewService(repo, nx, unionTTL)
	chHandler := characters.NewHandler(svc)

	api := e.Group("/api")
	api.Use(rateLimitMiddleware(deps.Config.RateLimit))
	api.GET("/characters/:nickname", chHandler.GetByNickname)
	api.PUT("/characters/:nickname/selection",
		chHandler.SaveSelection,
		middleware.BodyLimit(1<<14)) // 16KB

	return e
}

func rateLimitMiddleware(perMinute int) echo.MiddlewareFunc {
	perSecond := float64(perMinute) / 60.0

	burst := perMinute / 3
	if burst < 1 {
		burst = 1
	}

	store := middleware.NewRateLimiterMemoryStoreWithConfig(
		middleware.RateLimiterMemoryStoreConfig{
			Rate:      perSecond,
			Burst:     burst,
			ExpiresIn: 3 * time.Minute,
		},
	)

	return middleware.RateLimiterWithConfig(middleware.RateLimiterConfig{
		Store: store,
		IdentifierExtractor: func(c *echo.Context) (string, error) {
			return c.RealIP(), nil
		},
		ErrorHandler: func(_ *echo.Context, _ error) error {
			return echo.NewHTTPError(http.StatusForbidden, "rate limiter: cannot identify client")
		},
		DenyHandler: func(_ *echo.Context, _ string, _ error) error {
			return echo.NewHTTPError(http.StatusTooManyRequests, "rate limit exceeded")
		},
	})
}
