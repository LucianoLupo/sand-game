import { describe, it, expect, beforeEach } from "vitest";
import { SCENARIOS } from "./scenarios";
import { Species } from "./types";

const W = 300;
const H = 200;

// Captures all cells placed by a scenario build function
class GridCapture {
  grid: Int8Array;
  outOfBounds: { x: number; y: number }[] = [];

  constructor(
    private w: number,
    private h: number,
  ) {
    this.grid = new Int8Array(w * h).fill(-1);
  }

  setCell = (x: number, y: number, species: number): void => {
    if (x < 0 || x >= this.w || y < 0 || y >= this.h) {
      this.outOfBounds.push({ x, y });
      return;
    }
    this.grid[y * this.w + x] = species;
  };

  get(x: number, y: number): number {
    return this.grid[y * this.w + x];
  }

  count(species: number): number {
    let n = 0;
    for (const s of this.grid) if (s === species) n++;
    return n;
  }

  // Count species in a rectangular region (inclusive)
  countInRegion(species: number, x0: number, y0: number, x1: number, y1: number): number {
    let n = 0;
    for (let y = y0; y <= y1; y++) {
      for (let x = x0; x <= x1; x++) {
        if (this.get(x, y) === species) n++;
      }
    }
    return n;
  }

  // Check if an entire row segment is a given species
  isRowFilled(y: number, x0: number, x1: number, species: number): boolean {
    for (let x = x0; x <= x1; x++) {
      if (this.get(x, y) !== species) return false;
    }
    return true;
  }

  // Check if an entire column segment is a given species
  isColFilled(x: number, y0: number, y1: number, species: number): boolean {
    for (let y = y0; y <= y1; y++) {
      if (this.get(x, y) !== species) return false;
    }
    return true;
  }

  // Check if there's a gap (non-wall cell) in a row segment
  hasGapInRow(y: number, x0: number, x1: number, wallSpecies: number): boolean {
    for (let x = x0; x <= x1; x++) {
      if (this.get(x, y) !== wallSpecies) return true;
    }
    return false;
  }
}

// --- Shared tests for all scenarios ---

describe.each(Object.keys(SCENARIOS))("scenario '%s' common checks", (key) => {
  let grid: GridCapture;
  let faucets: ReturnType<(typeof SCENARIOS)[string]["build"]>;

  beforeEach(() => {
    grid = new GridCapture(W, H);
    faucets = SCENARIOS[key].build(W, H, grid.setCell);
  });

  it("builds without throwing", () => {
    // build was called in beforeEach — if it threw, we'd never get here
    expect(true).toBe(true);
  });

  it("places no cells out of bounds", () => {
    expect(grid.outOfBounds).toEqual([]);
  });

  it("places at least some cells", () => {
    const placed = grid.grid.filter((s) => s !== -1).length;
    expect(placed).toBeGreaterThan(0);
  });

  it("returns faucets within bounds", () => {
    for (const f of faucets) {
      expect(f.x).toBeGreaterThanOrEqual(0);
      expect(f.x).toBeLessThan(W);
      expect(f.y).toBeGreaterThanOrEqual(0);
      expect(f.y).toBeLessThan(H);
    }
  });

  it("returns faucets with valid size", () => {
    for (const f of faucets) {
      expect(f.size).toBeGreaterThanOrEqual(1);
    }
  });

  it("returns faucets with valid species (not Empty)", () => {
    for (const f of faucets) {
      expect(f.species).not.toBe(Species.Empty);
    }
  });
});

// --- Water Cycle ---

describe("water-cycle", () => {
  let grid: GridCapture;
  let faucets: ReturnType<(typeof SCENARIOS)[string]["build"]>;

  const l = 40,
    r = W - 41,
    t = 10,
    b = H - 6;

  beforeEach(() => {
    grid = new GridCapture(W, H);
    faucets = SCENARIOS["water-cycle"].build(W, H, grid.setCell);
  });

  it("returns 5 faucets (2 lava, 3 water)", () => {
    expect(faucets).toHaveLength(5);
    const lavaFaucets = faucets.filter((f) => f.species === Species.Lava);
    const waterFaucets = faucets.filter((f) => f.species === Species.Water);
    expect(lavaFaucets).toHaveLength(2);
    expect(waterFaucets).toHaveLength(3);
  });

  it("lava faucets are near the bottom, water faucets near the top", () => {
    const lavaFaucets = faucets.filter((f) => f.species === Species.Lava);
    const waterFaucets = faucets.filter((f) => f.species === Species.Water);
    for (const f of lavaFaucets) {
      expect(f.y).toBeGreaterThan(b - 10);
    }
    for (const f of waterFaucets) {
      expect(f.y).toBeLessThan(t + 10);
    }
  });

  it("faucets are inside the container", () => {
    for (const f of faucets) {
      expect(f.x).toBeGreaterThan(l + 1);
      expect(f.x).toBeLessThan(r - 1);
      expect(f.y).toBeGreaterThan(t + 1);
      expect(f.y).toBeLessThan(b - 1);
    }
  });

  it("has wall enclosure on all 4 sides (outer rectangle)", () => {
    expect(grid.isRowFilled(t, l, r, Species.Wall)).toBe(true);
    expect(grid.isRowFilled(b, l, r, Species.Wall)).toBe(true);
    expect(grid.isColFilled(l, t, b, Species.Wall)).toBe(true);
    expect(grid.isColFilled(r, t, b, Species.Wall)).toBe(true);
  });

  it("has double-thick walls (inner rectangle)", () => {
    expect(grid.isRowFilled(t + 1, l + 1, r - 1, Species.Wall)).toBe(true);
    expect(grid.isRowFilled(b - 1, l + 1, r - 1, Species.Wall)).toBe(true);
    expect(grid.isColFilled(l + 1, t + 1, b - 1, Species.Wall)).toBe(true);
    expect(grid.isColFilled(r - 1, t + 1, b - 1, Species.Wall)).toBe(true);
  });

  it("has lava at the bottom of the container", () => {
    const lavaCount = grid.countInRegion(Species.Lava, l + 2, b - 10, r - 2, b - 2);
    expect(lavaCount).toBeGreaterThan(0);
    const bottomBandArea = (r - 2 - (l + 2) + 1) * (b - 2 - (b - 10) + 1);
    expect(lavaCount).toBe(bottomBandArea);
  });

  it("has no initial water (water comes from faucets)", () => {
    const waterCount = grid.count(Species.Water);
    expect(waterCount).toBe(0);
  });

  it("has air space above lava for the water cycle", () => {
    const emptyAbove = grid.countInRegion(-1, l + 2, t + 2, r - 2, b - 11);
    expect(emptyAbove).toBeGreaterThan(0);
  });
});

// --- Rain Machine ---

describe("rain-machine", () => {
  let grid: GridCapture;
  let faucets: ReturnType<(typeof SCENARIOS)[string]["build"]>;

  const l = 25,
    r = W - 26;

  beforeEach(() => {
    grid = new GridCapture(W, H);
    faucets = SCENARIOS["rain-machine"].build(W, H, grid.setCell);
  });

  it("returns multiple steam faucets", () => {
    expect(faucets.length).toBeGreaterThanOrEqual(5);
    for (const f of faucets) {
      expect(f.species).toBe(Species.Steam);
    }
  });

  it("faucets are near the top of the container", () => {
    for (const f of faucets) {
      expect(f.y).toBeLessThan(30);
    }
  });

  it("has side walls spanning full height", () => {
    // Left wall (double thick)
    expect(grid.isColFilled(l, 12, H - 5, Species.Wall)).toBe(true);
    expect(grid.isColFilled(l + 1, 12, H - 5, Species.Wall)).toBe(true);
    // Right wall (double thick)
    expect(grid.isColFilled(r, 12, H - 5, Species.Wall)).toBe(true);
    expect(grid.isColFilled(r - 1, 12, H - 5, Species.Wall)).toBe(true);
  });

  it("has top and bottom caps", () => {
    expect(grid.isRowFilled(12, l, r, Species.Wall)).toBe(true);
    expect(grid.isRowFilled(H - 5, l, r, Species.Wall)).toBe(true);
  });

  it("has 2 terrace platforms at expected y levels", () => {
    const platformYs = [75, 135];
    for (const y of platformYs) {
      const wallCount = grid.countInRegion(Species.Wall, l + 2, y, r - 2, y + 1);
      expect(wallCount).toBeGreaterThan(0);
    }
  });

  it("each platform has a gap for water to flow through", () => {
    const platformYs = [75, 135];
    for (const y of platformYs) {
      expect(grid.hasGapInRow(y, l + 2, r - 2, Species.Wall)).toBe(true);
    }
  });

  it("platforms have alternating gap sides", () => {
    const gapWidth = 120;
    const pLeft = l + 2;
    const pRight = r - 2;

    // Platform at y=75: gap on right
    const rightGapWall75 = grid.countInRegion(Species.Wall, pRight - gapWidth + 1, 75, pRight, 75);
    expect(rightGapWall75).toBe(0);

    // Platform at y=135: gap on left
    const leftGapWall135 = grid.countInRegion(Species.Wall, pLeft, 135, pLeft + gapWidth - 1, 135);
    expect(leftGapWall135).toBe(0);
  });
});

// --- Ecosystem ---

describe("ecosystem", () => {
  let grid: GridCapture;
  let faucets: ReturnType<(typeof SCENARIOS)[string]["build"]>;

  const l = 20,
    r = W - 21,
    t = 10,
    b = H - 6;

  beforeEach(() => {
    grid = new GridCapture(W, H);
    faucets = SCENARIOS["ecosystem"].build(W, H, grid.setCell);
  });

  it("returns exactly 2 faucets (1 lava, 1 water)", () => {
    expect(faucets).toHaveLength(2);
    const species = faucets.map((f) => f.species).sort();
    expect(species).toContain(Species.Lava);
    expect(species).toContain(Species.Water);
  });

  it("has wall enclosure", () => {
    expect(grid.isRowFilled(t, l, r, Species.Wall)).toBe(true);
    expect(grid.isRowFilled(b, l, r, Species.Wall)).toBe(true);
    expect(grid.isColFilled(l, t, b, Species.Wall)).toBe(true);
    expect(grid.isColFilled(r, t, b, Species.Wall)).toBe(true);
  });

  it("has lava in the left zone", () => {
    const lavaLeft = grid.countInRegion(Species.Lava, l + 2, t + 2, l + 60, b - 2);
    expect(lavaLeft).toBeGreaterThan(100);
  });

  it("has water in the right zone", () => {
    const waterRight = grid.countInRegion(Species.Water, r - 60, t + 2, r - 2, b - 2);
    expect(waterRight).toBeGreaterThan(100);
  });

  it("has ice cap on water reservoir", () => {
    const iceCount = grid.count(Species.Ice);
    expect(iceCount).toBeGreaterThan(0);
    // Ice should be in the right zone only
    const iceRight = grid.countInRegion(Species.Ice, r - 60, t + 2, r - 2, b - 2);
    expect(iceRight).toBe(iceCount);
  });

  it("has plants in the center zone", () => {
    const lavaRight = l + 55;
    const waterLeft = r - 55;
    const gardenL = lavaRight + 5;
    const gardenR = waterLeft - 5;
    const plantCount = grid.countInRegion(Species.Plant, gardenL, t + 2, gardenR, b - 2);
    expect(plantCount).toBeGreaterThan(20);
  });

  it("has wood floor and trunks in center", () => {
    const lavaRight = l + 55;
    const waterLeft = r - 55;
    const gardenL = lavaRight + 5;
    const gardenR = waterLeft - 5;
    const woodCount = grid.countInRegion(Species.Wood, gardenL, t + 2, gardenR, b - 2);
    expect(woodCount).toBeGreaterThan(50);
  });

  it("has stone ground floor", () => {
    const stoneFloor = grid.countInRegion(Species.Stone, l + 2, b - 3, r - 2, b - 2);
    expect(stoneFloor).toBeGreaterThan(0);
  });

  it("no lava in the right zone or center", () => {
    const lavaRight = l + 55;
    // Lava should not appear to the right of the stone divider
    const lavaOutside = grid.countInRegion(Species.Lava, lavaRight + 2, t + 2, r - 2, b - 2);
    expect(lavaOutside).toBe(0);
  });
});

// --- Hourglass ---

describe("hourglass", () => {
  let grid: GridCapture;
  let faucets: ReturnType<(typeof SCENARIOS)[string]["build"]>;

  const cx = Math.floor(W / 2);
  const topY = 15;
  const botY = H - 16;
  const midY = Math.floor(H / 2);
  const topHalf = 80;

  beforeEach(() => {
    grid = new GridCapture(W, H);
    faucets = SCENARIOS["hourglass"].build(W, H, grid.setCell);
  });

  it("returns no faucets", () => {
    expect(faucets).toHaveLength(0);
  });

  it("has top and bottom caps", () => {
    // Top cap (2 rows thick)
    const topCapWalls = grid.countInRegion(Species.Wall, cx - topHalf - 1, topY, cx + topHalf + 1, topY + 1);
    expect(topCapWalls).toBe((topHalf * 2 + 3) * 2);

    // Bottom cap (2 rows thick)
    const botCapWalls = grid.countInRegion(Species.Wall, cx - topHalf - 1, botY - 1, cx + topHalf + 1, botY);
    expect(botCapWalls).toBe((topHalf * 2 + 3) * 2);
  });

  it("fills top chamber with sand", () => {
    const sandCount = grid.count(Species.Sand);
    expect(sandCount).toBeGreaterThan(1000);
  });

  it("has no sand in the bottom chamber", () => {
    const sandBelow = grid.countInRegion(Species.Sand, 0, midY + 1, W - 1, H - 1);
    expect(sandBelow).toBe(0);
  });

  it("neck is passable at midY (gap between walls at center)", () => {
    // At the neck, center cells should NOT be wall
    let gapCells = 0;
    for (let x = cx - 5; x <= cx + 5; x++) {
      if (grid.get(x, midY) !== Species.Wall) gapCells++;
    }
    // There should be a gap of at least 2 cells for sand to flow through
    expect(gapCells).toBeGreaterThanOrEqual(2);
  });

  it("walls are symmetric around center x", () => {
    // Sample several y values and check symmetry
    const testYs = [topY + 10, topY + 30, midY - 5, midY + 5, botY - 30, botY - 10];
    for (const y of testYs) {
      for (let dx = 0; dx <= topHalf + 2; dx++) {
        const leftCell = grid.get(cx - dx, y);
        const rightCell = grid.get(cx + dx, y);
        expect(leftCell).toBe(rightCell);
      }
    }
  });

  it("walls converge toward the neck", () => {
    // Near top: walls should be far from center
    const topWallX = findLeftWall(grid, topY + 5, cx);
    // Near neck: walls should be close to center
    const neckWallX = findLeftWall(grid, midY, cx);
    // Wall at top should be further from center than at neck
    expect(topWallX).toBeLessThan(neckWallX);
  });

  it("sand region narrows toward the neck", () => {
    // Count sand width near the top vs near the neck
    const sandWidthTop = countSpeciesInRow(grid, topY + 5, Species.Sand);
    const sandWidthNearNeck = countSpeciesInRow(grid, midY - 3, Species.Sand);
    expect(sandWidthTop).toBeGreaterThan(sandWidthNearNeck);
  });
});

// --- Lava Lamp ---

describe("lava-lamp", () => {
  let grid: GridCapture;
  let faucets: ReturnType<(typeof SCENARIOS)[string]["build"]>;

  const cx = Math.floor(W / 2);
  const halfW = 25;
  const colL = cx - halfW,
    colR = cx + halfW;
  const t = 10,
    b = H - 6;

  beforeEach(() => {
    grid = new GridCapture(W, H);
    faucets = SCENARIOS["lava-lamp"].build(W, H, grid.setCell);
  });

  it("returns exactly 1 water faucet", () => {
    expect(faucets).toHaveLength(1);
    expect(faucets[0].species).toBe(Species.Water);
  });

  it("faucet is at center x", () => {
    expect(faucets[0].x).toBe(cx);
  });

  it("faucet is near the top of the column", () => {
    expect(faucets[0].y).toBeGreaterThan(t);
    expect(faucets[0].y).toBeLessThan(t + 10);
  });

  it("has wall enclosure forming a narrow column", () => {
    expect(grid.isRowFilled(t, colL, colR, Species.Wall)).toBe(true);
    expect(grid.isRowFilled(b, colL, colR, Species.Wall)).toBe(true);
    expect(grid.isColFilled(colL, t, b, Species.Wall)).toBe(true);
    expect(grid.isColFilled(colR, t, b, Species.Wall)).toBe(true);
  });

  it("has double-thick walls", () => {
    expect(grid.isColFilled(colL + 1, t + 1, b - 1, Species.Wall)).toBe(true);
    expect(grid.isColFilled(colR - 1, t + 1, b - 1, Species.Wall)).toBe(true);
    expect(grid.isRowFilled(t + 1, colL + 1, colR - 1, Species.Wall)).toBe(true);
    expect(grid.isRowFilled(b - 1, colL + 1, colR - 1, Species.Wall)).toBe(true);
  });

  it("has lava pool at the bottom of the column", () => {
    const lavaCount = grid.countInRegion(Species.Lava, colL + 2, b - 6, colR - 2, b - 2);
    const bottomArea = (colR - 2 - (colL + 2) + 1) * (b - 2 - (b - 6) + 1);
    expect(lavaCount).toBe(bottomArea);
  });

  it("has air space above lava for convection", () => {
    const emptyAbove = grid.countInRegion(-1, colL + 2, t + 2, colR - 2, b - 7);
    expect(emptyAbove).toBeGreaterThan(0);
  });

  it("column is narrow (much less than half the world width)", () => {
    const columnWidth = colR - colL + 1;
    expect(columnWidth).toBeLessThan(W / 3);
  });

  it("no cells placed outside the column area", () => {
    const leftEmpty = grid.countInRegion(-1, 0, 0, colL - 1, H - 1);
    const leftTotal = colL * H;
    expect(leftEmpty).toBe(leftTotal);

    const rightEmpty = grid.countInRegion(-1, colR + 1, 0, W - 1, H - 1);
    const rightTotal = (W - 1 - colR) * H;
    expect(rightEmpty).toBe(rightTotal);
  });
});

// --- Helpers ---

function findLeftWall(grid: GridCapture, y: number, cx: number): number {
  for (let x = 0; x < cx; x++) {
    if (grid.get(x, y) === Species.Wall) return x;
  }
  return cx;
}

function countSpeciesInRow(grid: GridCapture, y: number, species: number): number {
  let n = 0;
  for (let x = 0; x < W; x++) {
    if (grid.get(x, y) === species) n++;
  }
  return n;
}
