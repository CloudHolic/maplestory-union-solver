package nexon

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strconv"
	"time"
)

// DefaultBaseURL is the global NEXON Open API endpoint.
const DefaultBaseURL = "https://open.api.nexon.com"

// Sentinel errors returned by Client.
var (
	// ErrCharacterNotFound - NEXON could not resolve the requested nickname or OCID.
	// Maps to 404 .
	ErrCharacterNotFound = errors.New("nexon: character not found")

	// ErrRateLimited - NEXON throttled us. Maps to 503.
	ErrRateLimited = errors.New("nexon: rate limited by upstream")

	// ErrUpstreamUnavailable - NEXON is undergoing maintenance or data is being prepared.
	// Maps to 503 with a hint to retry later.
	ErrUpstreamUnavailable = errors.New("nexon: upstream unavailable")

	// ErrUpstreamMisconfigured - our API key, game id, or path is rejected by NEXON.
	// Maps to 500.
	ErrUpstreamMisconfigured = errors.New("nexon: upstream misconfigured")
)

// APIError captures a non-2xx response that did not match any specific sentinel.
type APIError struct {
	StatusCode int
	Name       string
	Message    string
}

// Error implements error.
func (e *APIError) Error() string {
	return fmt.Sprintf("nexon api: status=%d name=%s message=%s",
		e.StatusCode, e.Name, e.Message)
}

// Client is a NEXON Open API HTTP client.
type Client struct {
	baseURL string
	apiKey  string
	http    *http.Client
}

// NewClient constructs a NEXON client bound to the given API key.
func NewClient(apiKey string) *Client {
	transport := &http.Transport{
		MaxIdleConns:        10,
		MaxIdleConnsPerHost: 10, // all calls go to one host; reuse aggressively.
		IdleConnTimeout:     90 * time.Second,
	}

	return &Client{
		baseURL: DefaultBaseURL,
		apiKey:  apiKey,
		http: &http.Client{
			Timeout:   10 * time.Second,
			Transport: transport,
		},
	}
}

// GetOCID resolves a character nickname to its OCID.
func (c *Client) GetOCID(ctx context.Context, nickname string) (string, error) {
	q := url.Values{}
	q.Set("character_name", nickname)

	body, err := c.getRaw(ctx, "/maplestory/v1/id", q)
	if err != nil {
		return "", err
	}

	var resp rawIDResponse
	if err := json.Unmarshal(body, &resp); err != nil {
		return "", fmt.Errorf("decoding id response: %w", err)
	}

	return resp.OCID, nil
}

// GetUnionData fetches the union-raider payload for the given OCID.
func (c *Client) GetUnionData(ctx context.Context, ocid string) (*UnionData, error) {
	q := url.Values{}
	q.Set("ocid", ocid)

	body, err := c.getRaw(ctx, "/maplestory/v1/user/union-raider", q)
	if err != nil {
		return nil, err
	}

	var raw rawUnionResponse
	if err := json.Unmarshal(body, &raw); err != nil {
		return nil, fmt.Errorf("decoding union response: %w", err)
	}

	return extractUnionData(&raw)
}

func extractUnionData(raw *rawUnionResponse) (*UnionData, error) {
	rawPresets := [PresetCount]*rawPresetBody{
		raw.Preset1, raw.Preset2, raw.Preset3, raw.Preset4, raw.Preset5,
	}

	out := &UnionData{
		UsePresetNo: raw.UsePresetNo,
		Presets:     make([][]Block, PresetCount),
	}

	for i, p := range rawPresets {
		if p == nil {
			out.Presets[i] = []Block{}
			continue
		}

		blocks := make([]Block, 0, len(p.UnionBlock))
		for _, rb := range p.UnionBlock {
			level, err := strconv.Atoi(rb.BlockLevel)
			if err != nil {
				return nil, fmt.Errorf("parsing block_level %q: %w", rb.BlockLevel, err)
			}

			blocks = append(blocks, Block{
				Type:  rb.BlockType,
				Class: rb.BlockClass,
				Level: level,
			})
		}

		out.Presets[i] = blocks
	}

	return out, nil
}

func (c *Client) getRaw(ctx context.Context, path string, q url.Values) ([]byte, error) {
	u := c.baseURL + path
	if len(q) > 0 {
		u += "?" + q.Encode()
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, u, nil)
	if err != nil {
		return nil, fmt.Errorf("building request: %w", err)
	}
	req.Header.Set("x-nxopen-api-key", c.apiKey)
	req.Header.Set("Accept", "application/json")

	resp, err := c.http.Do(req)
	if err != nil {
		return nil, fmt.Errorf("calling nexon: %w", err)
	}

	defer func(Body io.ReadCloser) {
		_ = Body.Close()
	}(resp.Body)

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("reading response: %w", err)
	}

	if resp.StatusCode >= 200 && resp.StatusCode < 300 {
		return body, nil
	}

	return nil, classifyError(resp.StatusCode, body)
}

func classifyError(status int, body []byte) error {
	var wrapper struct {
		Error rawAPIError `json:"error"`
	}

	_ = json.Unmarshal(body, &wrapper)

	switch wrapper.Error.Name {
	case "OPENAPI00003", "OPENAPI00004":
		// Invalid identifier, missing/invalid parameter
		return ErrCharacterNotFound
	case "OPENAPI00005", "OPENAPI00006":
		// Invalid API key, invalid game/path
		return ErrUpstreamMisconfigured
	case "OPENAPI00007":
		// Rate limited
		return ErrRateLimited
	case "OPENAPI00009", "OPENAPI00010", "OPENAPI00011":
		// Data being prepared / game maintenance / API maintenance
		return ErrUpstreamUnavailable
	}

	// Fallback: classify by HTTP status.
	switch status {
	case http.StatusTooManyRequests:
		return ErrRateLimited
	case http.StatusServiceUnavailable:
		return ErrUpstreamUnavailable
	}

	return &APIError{
		StatusCode: status,
		Name:       wrapper.Error.Name,
		Message:    wrapper.Error.Message,
	}
}
