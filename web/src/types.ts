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
  Ice = 10,
  Smoke = 11,
  Acid = 12,
  Wood = 13,
}

export interface Faucet {
  x: number;
  y: number;
  species: Species;
  size: number;
}

// Color palette for rendering
export const COLORS: Record<number, [number, number, number]> = {
  [Species.Empty]: [26, 26, 46],
  [Species.Sand]: [230, 197, 136],
  [Species.Water]: [74, 144, 217],
  [Species.Oil]: [75, 50, 20],
  [Species.Wall]: [128, 128, 128],
  [Species.Fire]: [255, 100, 20],
  [Species.Plant]: [34, 139, 34],
  [Species.Steam]: [200, 210, 230],
  [Species.Lava]: [207, 16, 32],
  [Species.Stone]: [100, 100, 110],
  [Species.Ice]: [170, 220, 240],
  [Species.Smoke]: [80, 80, 90],
  [Species.Acid]: [100, 255, 50],
  [Species.Wood]: [139, 90, 43],
};
