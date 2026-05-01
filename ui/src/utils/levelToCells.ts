export type BlockSize = 1 | 2 | 3 | 4 | 5;

type Thresholds = readonly [number, number, number, number];

const STANDARD_TABLE: Thresholds = [100, 140, 200, 250];
const MAPLE_M_TABLE: Thresholds = [50, 70, 120, 250];

export function blockSizeFromLevel(level: number, mapleM = false): BlockSize {
	const table = mapleM ? MAPLE_M_TABLE : STANDARD_TABLE;
	const idx = table.findIndex(t => t > level);

	return (idx === -1 ? 5 : idx + 1) as BlockSize;
}
