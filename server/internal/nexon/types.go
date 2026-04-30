// Package nexon is a thin HTTP client for NEXON Open API endpoints
// used by the union solver.
package nexon

// Block is the minimal piece description the solver needs.
type Block struct {
	Type  string `json:"type"`
	Class string `json:"class"`
	Level int    `json:"level"`
}

// PresetCount is the number of preset slots NEXON exposes per character.
const PresetCount = 5

// UnionData is the extracted, normalized form of a union-raider response.
type UnionData struct {
	UsePresetNo int       `json:"usePresetNo"`
	Presets     [][]Block `json:"presets"`
}

type rawIDResponse struct {
	OCID string `json:"ocid"`
}

type rawUnionResponse struct {
	UsePresetNo int            `json:"usePresetNo"`
	Preset1     *rawPresetBody `json:"union_raider_preset_1"`
	Preset2     *rawPresetBody `json:"union_raider_preset_2"`
	Preset3     *rawPresetBody `json:"union_raider_preset_3"`
	Preset4     *rawPresetBody `json:"union_raider_preset_4"`
	Preset5     *rawPresetBody `json:"union_raider_preset_5"`
}

type rawPresetBody struct {
	UnionBlock []rawBlock `json:"union_block"`
}

type rawBlock struct {
	BlockType  string `json:"block_type"`
	BlockClass string `json:"block_class"`
	BlockLevel string `json:"block_level"` // NEXON returns level as a string
}

type rawAPIError struct {
	Name    string `json:"name"`
	Message string `json:"message"`
}
