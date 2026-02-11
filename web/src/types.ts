export const enum Species {
  Empty = 0,
  Sand = 1,
  Water = 2,
  Oil = 3,
  Wall = 4,
}

export interface Faucet {
  x: number;
  y: number;
  species: Species;
}

// Color palette for rendering
export const COLORS: Record<number, [number, number, number]> = {
  [Species.Empty]: [26, 26, 46],      // #1a1a2e
  [Species.Sand]: [230, 197, 136],     // #e6c588
  [Species.Water]: [74, 144, 217],     // #4a90d9
  [Species.Oil]: [75, 50, 20],         // #4b3214
  [Species.Wall]: [128, 128, 128],     // #808080
};
