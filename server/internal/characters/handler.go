// Package characters serves the character cache lookup endpoint, which is
// the single entry point users hit when they search a MapleStory nickname.
package characters

import (
	"encoding/json"
	"errors"
	"io"
	"net/http"

	"github.com/CloudHolic/maplestory-union-solver/server/internal/nexon"
	"github.com/labstack/echo/v5"
)

// Handler exposes character-related HTTP endpoints.
type Handler struct {
	svc *Service
}

// NewHandler constructs a Handler backed by the given service.
func NewHandler(svc *Service) *Handler {
	return &Handler{svc: svc}
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

	view, err := h.svc.GetByNickname(c.Request().Context(), nickname)
	if err != nil {
		return mapError(err)
	}

	return c.JSON(http.StatusOK, view)
}

// SaveSelection persists the user's last-known selection region for the nickname.
//
// PUT /api/characters/:nickname/selection
func (h *Handler) SaveSelection(c *echo.Context) error {
	nickname := c.Param("nickname")
	if nickname == "" {
		return echo.NewHTTPError(http.StatusBadRequest, "nickname required")
	}

	body, err := io.ReadAll(c.Request().Body)
	if err != nil {
		return echo.NewHTTPError(http.StatusBadRequest, "reading body")
	}

	if len(body) == 0 {
		return echo.NewHTTPError(http.StatusBadRequest, "empty body")
	}
	if !json.Valid(body) {
		return echo.NewHTTPError(http.StatusBadRequest, "body must be valid JSON")
	}

	if err := h.svc.SaveSelection(c.Request().Context(), nickname, string(body)); err != nil {
		if errors.Is(err, ErrNotFound) {
			return echo.NewHTTPError(http.StatusNotFound, "character not searched yet")
		}

		return echo.NewHTTPError(http.StatusInternalServerError, "internal server error")
	}

	return c.NoContent(http.StatusNoContent)
}

func mapError(err error) error {
	switch {
	case errors.Is(err, nexon.ErrCharacterNotFound):
		return echo.NewHTTPError(http.StatusNotFound, "character not found")
	case errors.Is(err, nexon.ErrRateLimited):
		return echo.NewHTTPError(http.StatusServiceUnavailable, "upstream rate limited, retry shortly")
	case errors.Is(err, nexon.ErrUpstreamUnavailable):
		return echo.NewHTTPError(http.StatusServiceUnavailable, "upstream temporarily unavailable")
	case errors.Is(err, nexon.ErrUpstreamMisconfigured):
		return echo.NewHTTPError(http.StatusInternalServerError, "upstream misconfigured")
	}

	return echo.NewHTTPError(http.StatusInternalServerError, "internal server error")
}
