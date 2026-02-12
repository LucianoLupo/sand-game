import { Species, type Faucet } from "./types";

type SetCellFn = (x: number, y: number, species: Species) => void;

export interface Scenario {
  name: string;
  build: (width: number, height: number, setCell: SetCellFn) => Faucet[];
}

function hLine(setCell: SetCellFn, y: number, x0: number, x1: number, species: Species) {
  for (let x = x0; x <= x1; x++) setCell(x, y, species);
}

function vLine(setCell: SetCellFn, x: number, y0: number, y1: number, species: Species) {
  for (let y = y0; y <= y1; y++) setCell(x, y, species);
}

function fillRect(setCell: SetCellFn, x0: number, y0: number, x1: number, y1: number, species: Species) {
  for (let y = y0; y <= y1; y++) {
    for (let x = x0; x <= x1; x++) {
      setCell(x, y, species);
    }
  }
}

function outlineRect(setCell: SetCellFn, x0: number, y0: number, x1: number, y1: number, species: Species) {
  hLine(setCell, y0, x0, x1, species);
  hLine(setCell, y1, x0, x1, species);
  vLine(setCell, x0, y0, y1, species);
  vLine(setCell, x1, y0, y1, species);
}

export const SCENARIOS: Record<string, Scenario> = {
  "water-cycle": {
    name: "Water Cycle",
    build(w, h, setCell) {
      const l = 40, r = w - 41, t = 10, b = h - 6;

      // Double-thick enclosed container
      outlineRect(setCell, l, t, r, b, Species.Wall);
      outlineRect(setCell, l + 1, t + 1, r - 1, b - 1, Species.Wall);

      // Lava pool at bottom
      fillRect(setCell, l + 2, b - 10, r - 2, b - 2, Species.Lava);

      // No initial water — water faucets drip from ceiling.
      // Water falls through air → hits lava → steam → rises → condenses → falls → cycle
      const cx = Math.floor((l + r) / 2);
      return [
        { x: cx - 40, y: b - 3, species: Species.Lava, size: 1 },
        { x: cx + 40, y: b - 3, species: Species.Lava, size: 1 },
        { x: cx - 30, y: t + 3, species: Species.Water, size: 1 },
        { x: cx, y: t + 3, species: Species.Water, size: 1 },
        { x: cx + 30, y: t + 3, species: Species.Water, size: 1 },
      ];
    },
  },

  "rain-machine": {
    name: "Rain Machine",
    build(w, h, setCell) {
      const l = 25, r = w - 26;

      // Enclosure: sides, top, bottom
      for (let i = 0; i < 2; i++) {
        vLine(setCell, l + i, 12, h - 5, Species.Wall);
        vLine(setCell, r - i, 12, h - 5, Species.Wall);
      }
      hLine(setCell, 12, l, r, Species.Wall);
      hLine(setCell, 13, l, r, Species.Wall);
      hLine(setCell, h - 5, l, r, Species.Wall);
      hLine(setCell, h - 6, l, r, Species.Wall);

      // Terraced platforms with alternating wide gaps
      const gapWidth = 120;
      const pLeft = l + 2;
      const pRight = r - 2;
      const platforms = [
        { y: 75, gapSide: "right" as const },
        { y: 135, gapSide: "left" as const },
      ];

      for (const p of platforms) {
        for (let x = pLeft; x <= pRight; x++) {
          const inGap =
            p.gapSide === "right"
              ? x > pRight - gapWidth
              : x < pLeft + gapWidth;
          if (!inGap) {
            setCell(x, p.y, Species.Wall);
            setCell(x, p.y + 1, Species.Wall);
          }
        }
      }

      // Steam faucets along the top
      const faucets: Faucet[] = [];
      for (let x = l + 20; x <= r - 20; x += 25) {
        faucets.push({ x, y: 18, species: Species.Steam, size: 2 });
      }
      return faucets;
    },
  },

  "ecosystem": {
    name: "Ecosystem",
    build(w, h, setCell) {
      const l = 20, r = w - 21, t = 10, b = h - 6;

      // Outer walls
      outlineRect(setCell, l, t, r, b, Species.Wall);
      outlineRect(setCell, l + 1, t + 1, r - 1, b - 1, Species.Wall);

      // Stone ground floor
      fillRect(setCell, l + 2, b - 3, r - 2, b - 2, Species.Stone);

      // Left: Lava pit with stone container
      const lavaRight = l + 55;
      vLine(setCell, lavaRight, b - 45, b - 4, Species.Stone);
      vLine(setCell, lavaRight + 1, b - 45, b - 4, Species.Stone);
      hLine(setCell, b - 45, l + 2, lavaRight + 1, Species.Stone);
      fillRect(setCell, l + 2, b - 44, lavaRight - 1, b - 4, Species.Lava);

      // Right: Water reservoir with stone container + ice cap
      const waterLeft = r - 55;
      vLine(setCell, waterLeft, b - 45, b - 4, Species.Stone);
      vLine(setCell, waterLeft - 1, b - 45, b - 4, Species.Stone);
      hLine(setCell, b - 45, waterLeft - 1, r - 2, Species.Stone);
      fillRect(setCell, waterLeft + 1, b - 44, r - 2, b - 4, Species.Water);
      fillRect(setCell, waterLeft + 1, b - 48, r - 2, b - 45, Species.Ice);

      // Center: Garden
      const gardenL = lavaRight + 5;
      const gardenR = waterLeft - 5;

      // Wood floor
      fillRect(setCell, gardenL, b - 6, gardenR, b - 4, Species.Wood);

      // Plant columns
      for (let x = gardenL + 3; x <= gardenR - 3; x += 6) {
        for (let dy = 0; dy < 18; dy++) {
          setCell(x, b - 7 - dy, Species.Plant);
        }
      }

      // Wood trunks with small canopies
      for (let x = gardenL + 12; x <= gardenR - 12; x += 20) {
        vLine(setCell, x, b - 25, b - 7, Species.Wood);
        hLine(setCell, b - 25, x - 3, x + 3, Species.Wood);
      }

      return [
        { x: l + 25, y: b - 5, species: Species.Lava, size: 1 },
        { x: r - 25, y: b - 5, species: Species.Water, size: 1 },
      ];
    },
  },

  "hourglass": {
    name: "Hourglass",
    build(w, h, setCell) {
      const cx = Math.floor(w / 2);
      const topY = 15;
      const botY = h - 16;
      const midY = Math.floor(h / 2);
      const topHalf = 80;
      const neckHalf = 3;

      // Top and bottom caps
      for (let dy = 0; dy < 2; dy++) {
        hLine(setCell, topY + dy, cx - topHalf - 1, cx + topHalf + 1, Species.Wall);
        hLine(setCell, botY - dy, cx - topHalf - 1, cx + topHalf + 1, Species.Wall);
      }

      // Converging/diverging diagonal walls
      for (let y = topY; y <= botY; y++) {
        let dist: number;
        if (y <= midY) {
          const t = (y - topY) / (midY - topY);
          dist = topHalf + (neckHalf - topHalf) * t;
        } else {
          const t = (y - midY) / (botY - midY);
          dist = neckHalf + (topHalf - neckHalf) * t;
        }

        const xl = cx - Math.round(dist);
        const xr = cx + Math.round(dist);
        setCell(xl, y, Species.Wall);
        setCell(xl - 1, y, Species.Wall);
        setCell(xr, y, Species.Wall);
        setCell(xr + 1, y, Species.Wall);
      }

      // Fill top chamber with sand
      for (let y = topY + 2; y < midY; y++) {
        const t = (y - topY) / (midY - topY);
        const dist = topHalf + (neckHalf - topHalf) * t;
        const xl = cx - Math.round(dist) + 2;
        const xr = cx + Math.round(dist) - 2;
        for (let x = xl; x <= xr; x++) {
          setCell(x, y, Species.Sand);
        }
      }

      return [];
    },
  },

  "lava-lamp": {
    name: "Lava Lamp",
    build(w, h, setCell) {
      const cx = Math.floor(w / 2);
      const halfW = 25;
      const l = cx - halfW, r = cx + halfW;
      const t = 10, b = h - 6;

      // Column walls (double thick)
      outlineRect(setCell, l, t, r, b, Species.Wall);
      outlineRect(setCell, l + 1, t + 1, r - 1, b - 1, Species.Wall);

      // Lava pool at bottom (5 rows)
      fillRect(setCell, l + 2, b - 6, r - 2, b - 2, Species.Lava);

      // Column is mostly air — water drips from top, hits lava, creates steam,
      // steam rises through air, condenses, water falls back down
      return [{ x: cx, y: t + 3, species: Species.Water, size: 1 }];
    },
  },
};
