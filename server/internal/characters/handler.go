// Package characters serves the character cache lookup endpoint, which is
// the single entry point users hit when they search a MapleStory nickname.
package characters

import (
	"net/http"

	"github.com/jmoiron/sqlx"
	"github.com/labstack/echo/v5"
)

// Handler bundles the dependencies required by the character HTTP handlers.
type Handler struct {
	db *sqlx.DB
}

// NewHandler constructs a Handler bound to the given database handle.
func NewHandler(db *sqlx.DB) *Handler {
	return &Handler{db: db}
}

// GetByNickname returns the cached character data, the user's last saved selection region,
// and the timestamp of the previous search for the given nickname.
//
// GET /api/characters/:nickname
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
