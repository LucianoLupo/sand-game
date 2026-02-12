export const enum Species {
  Empty = 0,
  Sand = 1,
  Water = 2,
  Oil = 3,
  Wall = 4,
  Fire = 5,
  Plant = 6,
  Steam = 7,
  Lava = 8,
  Stone = 9,
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
  [Species.Fire]: [255, 100, 20],      // #ff6414
  [Species.Plant]: [34, 139, 34],      // #228b22
  [Species.Steam]: [200, 210, 230],    // #c8d2e6
  [Species.Lava]: [207, 16, 32],       // #cf1020
  [Species.Stone]: [100, 100, 110],    // #64646e
};
