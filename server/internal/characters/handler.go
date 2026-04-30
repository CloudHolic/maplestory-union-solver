package characters

import (
	"net/http"

	"github.com/jmoiron/sqlx"
	"github.com/labstack/echo/v5"
)

type Handler struct {
	db *sqlx.DB
}

func NewHandler(db *sqlx.DB) *Handler {
	return &Handler{db: db}
}

func (h *Handler) GetByNickname(c *echo.Context) error {
	nickname := c.Param("nickname")
	if nickname == "" {
		return echo.NewHTTPError(http.StatusBadRequest, "nickname required")
	}

	// TODO: Search cache -> Call Nexon API if miss
	return c.JSON(http.StatusNotImplemented, map[string]string{
		"nickname": nickname,
		"status":   "handler stub - implementation pending"})
}

func (h *Handler) RecentSearches(c *echo.Context) error {
	limit, err := echo.QueryParamOr[int](c, "limit", 10)
	if err != nil {
		return echo.NewHTTPError(http.StatusBadRequest, "invalid limit")
	}

	limit = clamp(limit, 1, 50)

	// TODO: Search DB
	return c.JSON(http.StatusNotImplemented, map[string]any{
		"limit":   limit,
		"results": []any{},
		"status":  "handler stub - implementation pending"})
}

func clamp(v, lo, hi int) int {
	if v < lo {
		return lo
	}
	if v > hi {
		return hi
	}

	return v
}
