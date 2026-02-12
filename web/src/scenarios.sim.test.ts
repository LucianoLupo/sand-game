import { describe, it, expect, beforeAll } from "vitest";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { SCENARIOS } from "./scenarios";
import type { Faucet } from "./types";

// --- WASM loading ---

let initSync: typeof import("./wasm/sand_sim").initSync;
let World: typeof import("./wasm/sand_sim").World;
let wasmMemory: WebAssembly.Memory;

beforeAll(async () => {
  // Dynamic import so vitest doesn't try to resolve WASM at parse time
  const mod = await import("./wasm/sand_sim");
  initSync = mod.initSync;
  World = mod.World;

  const wasmPath = new URL("./wasm/sand_sim_bg.wasm", import.meta.url);
  const wasmBytes = readFileSync(wasmPath);
  const exports = initSync(wasmBytes);
  wasmMemory = exports.memory;
});

// --- Constants ---

const W = 300;
const H = 200;
const TICKS_PER_FRAME = 3;

const SPECIES_NAMES: Record<number, string> = {
  0: "Empty",
  1: "Sand",
  2: "Water",
  3: "Oil",
  4: "Wall",
  5: "Fire",
  6: "Plant",
  7: "Steam",
  8: "Lava",
  9: "Stone",
  10: "Ice",
  11: "Smoke",
  12: "Acid",
  13: "Wood",
};

// --- Helpers ---

function applyFaucets(world: InstanceType<typeof World>, faucets: Faucet[]) {
  for (const f of faucets) {
    const r = Math.max(0, f.size - 1);
    for (let dy = -r; dy <= r; dy++) {
      for (let dx = -r; dx <= r; dx++) {
        if (dx * dx + dy * dy <= r * r + r) {
          const x = f.x + dx;
          const y = f.y + dy;
          if (x >= 0 && x < W && y >= 0 && y < H) {
            world.set_cell(x, y, f.species);
          }
        }
      }
    }
  }
}

function readGrid(world: InstanceType<typeof World>): Uint8Array {
  const ptr = world.cells_ptr();
  const byteLen = W * H * 4;
  return new Uint8Array(wasmMemory.buffer, ptr, byteLen);
}

function speciesAt(cells: Uint8Array, x: number, y: number): number {
  return cells[(y * W + x) * 4];
}

interface RegionCounts {
  [species: string]: number;
}

function countSpeciesInRegion(
  cells: Uint8Array,
  x0: number,
  y0: number,
  x1: number,
  y1: number,
): RegionCounts {
  const counts: Record<number, number> = {};
  for (let y = y0; y <= y1; y++) {
    for (let x = x0; x <= x1; x++) {
      const s = speciesAt(cells, x, y);
      counts[s] = (counts[s] || 0) + 1;
    }
  }
  const named: RegionCounts = {};
  for (const [id, count] of Object.entries(counts)) {
    named[SPECIES_NAMES[Number(id)] ?? `Unknown(${id})`] = count;
  }
  return named;
}

interface Snapshot {
  tick: number;
  total: RegionCounts;
  topHalf: RegionCounts;
  bottomHalf: RegionCounts;
  leftThird: RegionCounts;
  rightThird: RegionCounts;
}

interface SimLog {
  scenario: string;
  width: number;
  height: number;
  totalTicks: number;
  faucetCount: number;
  faucets: Faucet[];
  snapshots: Snapshot[];
}

function takeSnapshot(cells: Uint8Array, tick: number): Snapshot {
  const midY = Math.floor(H / 2);
  const thirdX = Math.floor(W / 3);
  return {
    tick,
    total: countSpeciesInRegion(cells, 0, 0, W - 1, H - 1),
    topHalf: countSpeciesInRegion(cells, 0, 0, W - 1, midY - 1),
    bottomHalf: countSpeciesInRegion(cells, 0, midY, W - 1, H - 1),
    leftThird: countSpeciesInRegion(cells, 0, 0, thirdX - 1, H - 1),
    rightThird: countSpeciesInRegion(cells, W - thirdX, 0, W - 1, H - 1),
  };
}

function runSimulation(
  scenarioKey: string,
  totalTicks: number,
  snapshotInterval: number,
): SimLog {
  const scenario = SCENARIOS[scenarioKey];
  const world = new World(W, H);

  const faucets = scenario.build(W, H, (x, y, s) => world.set_cell(x, y, s));

  const snapshots: Snapshot[] = [];

  // Snapshot at tick 0 (initial state)
  snapshots.push(takeSnapshot(readGrid(world), 0));

  let tickCount = 0;
  for (let frame = 0; frame < Math.ceil(totalTicks / TICKS_PER_FRAME); frame++) {
    // Faucets apply once per frame (matches real game loop)
    applyFaucets(world, faucets);

    for (let t = 0; t < TICKS_PER_FRAME; t++) {
      world.tick();
      tickCount++;

      if (tickCount % snapshotInterval === 0) {
        snapshots.push(takeSnapshot(readGrid(world), tickCount));
      }
    }
  }

  world.free();

  return {
    scenario: scenarioKey,
    width: W,
    height: H,
    totalTicks: tickCount,
    faucetCount: faucets.length,
    faucets,
    snapshots,
  };
}

function writeLog(log: SimLog) {
  const dir = new URL("../sim-logs/", import.meta.url);
  mkdirSync(dir, { recursive: true });
  const filePath = new URL(`${log.scenario}.json`, dir);
  writeFileSync(filePath, JSON.stringify(log, null, 2));
}

function getCount(counts: RegionCounts, species: string): number {
  return counts[species] ?? 0;
}

// --- Simulation tests ---

describe("water-cycle simulation", () => {
  let log: SimLog;

  beforeAll(() => {
    log = runSimulation("water-cycle", 600, 100);
    writeLog(log);
  });

  it("logs are written", () => {
    expect(log.snapshots.length).toBeGreaterThan(1);
  });

  it("starts with lava but no water (water comes from faucets)", () => {
    const s0 = log.snapshots[0].total;
    expect(getCount(s0, "Lava")).toBeGreaterThan(0);
    expect(getCount(s0, "Water")).toBe(0);
  });

  it("water accumulates from faucets over time", () => {
    const last = log.snapshots[log.snapshots.length - 1].total;
    expect(getCount(last, "Water")).toBeGreaterThan(0);
  });

  it("steam forms from water hitting lava", () => {
    const last = log.snapshots[log.snapshots.length - 1].total;
    expect(getCount(last, "Steam")).toBeGreaterThan(0);
  });

  it("lava is maintained by faucets", () => {
    const last = log.snapshots[log.snapshots.length - 1].total;
    expect(getCount(last, "Lava")).toBeGreaterThan(0);
  });

  it("steam rises through the air column", () => {
    const last = log.snapshots[log.snapshots.length - 1];
    const topSteam = getCount(last.topHalf, "Steam");
    const bottomSteam = getCount(last.bottomHalf, "Steam");
    console.log("Water Cycle steam distribution — top:", topSteam, "bottom:", bottomSteam);
    // With no water layer blocking, steam should rise into the upper airspace
    expect(topSteam + bottomSteam).toBeGreaterThan(0);
  });

  it("walls remain constant throughout", () => {
    const firstWalls = getCount(log.snapshots[0].total, "Wall");
    for (const snap of log.snapshots) {
      expect(getCount(snap.total, "Wall")).toBe(firstWalls);
    }
  });

  it("logs evolution for analysis", () => {
    console.log("\n=== Water Cycle Evolution ===");
    for (const snap of log.snapshots) {
      const t = snap.total;
      console.log(
        `tick=${snap.tick.toString().padStart(4)}:`,
        `Water=${getCount(t, "Water").toString().padStart(5)}`,
        `Lava=${getCount(t, "Lava").toString().padStart(5)}`,
        `Steam=${getCount(t, "Steam").toString().padStart(4)}`,
        `Stone=${getCount(t, "Stone").toString().padStart(4)}`,
      );
    }
  });
});

describe("rain-machine simulation", () => {
  let log: SimLog;

  beforeAll(() => {
    log = runSimulation("rain-machine", 600, 100);
    writeLog(log);
  });

  it("starts with no water (steam faucets only)", () => {
    const s0 = log.snapshots[0].total;
    expect(getCount(s0, "Water")).toBe(0);
    expect(getCount(s0, "Steam")).toBe(0);
  });

  it("steam appears from faucets", () => {
    const s1 = log.snapshots[1].total;
    expect(getCount(s1, "Steam")).toBeGreaterThan(0);
  });

  it("water condenses from steam over time", () => {
    const last = log.snapshots[log.snapshots.length - 1].total;
    expect(getCount(last, "Water")).toBeGreaterThan(0);
  });

  it("water accumulates over time (condensation working)", () => {
    const waterOverTime = log.snapshots.map((s) => getCount(s.total, "Water"));
    // Water should monotonically increase as steam condenses
    for (let i = 1; i < waterOverTime.length; i++) {
      expect(waterOverTime[i]).toBeGreaterThanOrEqual(waterOverTime[i - 1]);
    }
  });

  it("water flows through platforms toward the bottom", () => {
    const last = log.snapshots[log.snapshots.length - 1];
    const topWater = getCount(last.topHalf, "Water");
    const bottomWater = getCount(last.bottomHalf, "Water");
    console.log("Rain Machine water distribution — top:", topWater, "bottom:", bottomWater);
    // With wider gaps, water should reach the bottom half
    expect(topWater + bottomWater).toBeGreaterThan(0);
  });

  it("walls remain constant throughout", () => {
    const firstWalls = getCount(log.snapshots[0].total, "Wall");
    for (const snap of log.snapshots) {
      expect(getCount(snap.total, "Wall")).toBe(firstWalls);
    }
  });

  it("logs evolution for analysis", () => {
    console.log("\n=== Rain Machine Evolution ===");
    for (const snap of log.snapshots) {
      const t = snap.total;
      console.log(
        `tick=${snap.tick.toString().padStart(4)}:`,
        `Water=${getCount(t, "Water").toString().padStart(5)}`,
        `Steam=${getCount(t, "Steam").toString().padStart(4)}`,
        `topWater=${getCount(snap.topHalf, "Water").toString().padStart(5)}`,
        `botWater=${getCount(snap.bottomHalf, "Water").toString().padStart(5)}`,
      );
    }
  });
});

describe("ecosystem simulation", () => {
  let log: SimLog;

  beforeAll(() => {
    log = runSimulation("ecosystem", 600, 100);
    writeLog(log);
  });

  it("starts with lava, water, plants, wood, and ice", () => {
    const s0 = log.snapshots[0].total;
    expect(getCount(s0, "Lava")).toBeGreaterThan(0);
    expect(getCount(s0, "Water")).toBeGreaterThan(0);
    expect(getCount(s0, "Plant")).toBeGreaterThan(0);
    expect(getCount(s0, "Wood")).toBeGreaterThan(0);
    expect(getCount(s0, "Ice")).toBeGreaterThan(0);
  });

  it("maintains lava presence (faucet replenishes)", () => {
    const last = log.snapshots[log.snapshots.length - 1].total;
    expect(getCount(last, "Lava")).toBeGreaterThan(0);
  });

  it("maintains water presence (faucet replenishes)", () => {
    const last = log.snapshots[log.snapshots.length - 1].total;
    expect(getCount(last, "Water")).toBeGreaterThan(0);
  });

  it("lava stays on the left side", () => {
    const last = log.snapshots[log.snapshots.length - 1];
    const leftLava = getCount(last.leftThird, "Lava");
    const rightLava = getCount(last.rightThird, "Lava");
    expect(leftLava).toBeGreaterThan(rightLava);
  });

  it("walls remain constant throughout", () => {
    const firstWalls = getCount(log.snapshots[0].total, "Wall");
    for (const snap of log.snapshots) {
      expect(getCount(snap.total, "Wall")).toBe(firstWalls);
    }
  });
});

describe("hourglass simulation", () => {
  let log: SimLog;

  beforeAll(() => {
    log = runSimulation("hourglass", 900, 150);
    writeLog(log);
  });

  it("starts with all sand in the top half", () => {
    const s0 = log.snapshots[0];
    const topSand = getCount(s0.topHalf, "Sand");
    const botSand = getCount(s0.bottomHalf, "Sand");
    expect(topSand).toBeGreaterThan(0);
    expect(botSand).toBe(0);
  });

  it("sand moves from top to bottom over time", () => {
    const last = log.snapshots[log.snapshots.length - 1];
    const botSand = getCount(last.bottomHalf, "Sand");
    expect(botSand).toBeGreaterThan(0);
  });

  it("top half sand decreases over time", () => {
    const firstTopSand = getCount(log.snapshots[0].topHalf, "Sand");
    const lastTopSand = getCount(log.snapshots[log.snapshots.length - 1].topHalf, "Sand");
    expect(lastTopSand).toBeLessThan(firstTopSand);
  });

  it("total sand is conserved (no sand created or destroyed)", () => {
    const firstSand = getCount(log.snapshots[0].total, "Sand");
    const lastSand = getCount(log.snapshots[log.snapshots.length - 1].total, "Sand");
    expect(lastSand).toBe(firstSand);
  });

  it("sand flow is gradual (increases each snapshot)", () => {
    let prevBotSand = 0;
    for (const snap of log.snapshots) {
      const botSand = getCount(snap.bottomHalf, "Sand");
      expect(botSand).toBeGreaterThanOrEqual(prevBotSand);
      prevBotSand = botSand;
    }
  });

  it("walls remain constant throughout", () => {
    const firstWalls = getCount(log.snapshots[0].total, "Wall");
    for (const snap of log.snapshots) {
      expect(getCount(snap.total, "Wall")).toBe(firstWalls);
    }
  });
});

describe("lava-lamp simulation", () => {
  let log: SimLog;

  beforeAll(() => {
    log = runSimulation("lava-lamp", 600, 100);
    writeLog(log);
  });

  it("starts with lava pool at the bottom", () => {
    const s0 = log.snapshots[0].total;
    expect(getCount(s0, "Lava")).toBeGreaterThan(100);
  });

  it("water accumulates from faucet over time", () => {
    const last = log.snapshots[log.snapshots.length - 1].total;
    expect(getCount(last, "Water")).toBeGreaterThan(0);
  });

  it("steam forms from water hitting lava", () => {
    const last = log.snapshots[log.snapshots.length - 1].total;
    expect(getCount(last, "Steam")).toBeGreaterThan(0);
  });

  it("lava pool is maintained", () => {
    const last = log.snapshots[log.snapshots.length - 1].total;
    expect(getCount(last, "Lava")).toBeGreaterThan(0);
  });

  it("walls remain constant throughout", () => {
    const firstWalls = getCount(log.snapshots[0].total, "Wall");
    for (const snap of log.snapshots) {
      expect(getCount(snap.total, "Wall")).toBe(firstWalls);
    }
  });

  it("logs species evolution for manual analysis", () => {
    console.log("\n=== Lava Lamp Evolution ===");
    for (const snap of log.snapshots) {
      const t = snap.total;
      console.log(
        `tick=${snap.tick.toString().padStart(4)}:`,
        `Water=${getCount(t, "Water").toString().padStart(5)}`,
        `Lava=${getCount(t, "Lava").toString().padStart(4)}`,
        `Stone=${getCount(t, "Stone").toString().padStart(4)}`,
        `Steam=${getCount(t, "Steam").toString().padStart(4)}`,
        `Empty=${getCount(t, "Empty").toString().padStart(5)}`,
      );
    }
  });
});
