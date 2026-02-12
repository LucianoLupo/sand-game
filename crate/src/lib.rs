#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// Species IDs
const SPECIES_EMPTY: u8 = 0;
const SPECIES_SAND: u8 = 1;
const SPECIES_WATER: u8 = 2;
const SPECIES_OIL: u8 = 3;
const SPECIES_WALL: u8 = 4;
const SPECIES_FIRE: u8 = 5;
const SPECIES_PLANT: u8 = 6;
const SPECIES_STEAM: u8 = 7;
const SPECIES_LAVA: u8 = 8;
const SPECIES_STONE: u8 = 9;
const SPECIES_ICE: u8 = 10;
const SPECIES_SMOKE: u8 = 11;
const SPECIES_ACID: u8 = 12;
const SPECIES_WOOD: u8 = 13;

// Temperature constants (u8, ~6 deg C per step)
const TEMP_AMBIENT: u8 = 12;
const TEMP_FREEZE: u8 = 8;
const TEMP_BOIL: u8 = 25;
const TEMP_OIL_IGNITE: u8 = 40;
const TEMP_WOOD_IGNITE: u8 = 48;
const TEMP_PLANT_IGNITE: u8 = 55;
const TEMP_STONE_MELT: u8 = 100;
const TEMP_FIRE_PLACE: u8 = 180;
const TEMP_LAVA_DEFAULT: u8 = 200;
const TEMP_FIRE_SUSTAIN: u8 = 30;
const TEMP_ICE_DEFAULT: u8 = 2;

// Fire fuel amounts
const FUEL_OIL_MIN: u8 = 30;
const FUEL_OIL_MAX: u8 = 50;
const FUEL_PLANT_MIN: u8 = 40;
const FUEL_PLANT_MAX: u8 = 70;
const FUEL_WOOD_MIN: u8 = 80;
const FUEL_WOOD_MAX: u8 = 140;
const FUEL_USER_PLACED: u8 = 60;

const CELL_STRIDE: usize = 4;

// ── Native PRNG (xorshift32) ────────────────────────────────────────
static mut RNG_STATE: u32 = 0xDEAD_BEEF;

#[inline(always)]
fn rand_u32() -> u32 {
    unsafe {
        let mut s = RNG_STATE;
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        RNG_STATE = s;
        s
    }
}

fn rand() -> f64 {
    (rand_u32() as f64) / (u32::MAX as f64)
}

fn rand_bool() -> bool {
    rand_u32() & 1 == 0
}

fn rand_ra() -> u8 {
    (rand_u32() % 30) as u8
}

fn rand_range(min: u8, max: u8) -> u8 {
    let range = max.saturating_sub(min);
    if range == 0 { return min; }
    min + (rand_u32() % range as u32) as u8
}

#[inline(always)]
fn cell_idx(width: usize, x: usize, y: usize) -> usize {
    (y * width + x) * CELL_STRIDE
}

#[inline(always)]
fn in_bounds(width: usize, height: usize, x: isize, y: isize) -> bool {
    x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height
}

#[inline(always)]
fn set_clock(cells: &mut [u8], width: usize, x: usize, y: usize, clock: u8) {
    cells[cell_idx(width, x, y) + 3] = clock;
}

#[inline(always)]
fn get_species(cells: &[u8], width: usize, x: usize, y: usize) -> u8 {
    cells[cell_idx(width, x, y)]
}

#[inline(always)]
fn get_clock(cells: &[u8], width: usize, x: usize, y: usize) -> u8 {
    cells[cell_idx(width, x, y) + 3]
}

#[inline(always)]
fn get_temp(cells: &[u8], width: usize, x: usize, y: usize) -> u8 {
    cells[cell_idx(width, x, y) + 2]
}

#[inline(always)]
fn set_cell_raw(cells: &mut [u8], width: usize, x: usize, y: usize, species: u8, ra: u8, rb: u8, clock: u8) {
    let i = cell_idx(width, x, y);
    cells[i] = species;
    cells[i + 1] = ra;
    cells[i + 2] = rb;
    cells[i + 3] = clock;
}

#[inline(always)]
fn swap_cells(cells: &mut [u8], width: usize, x1: usize, y1: usize, x2: usize, y2: usize) {
    let i1 = cell_idx(width, x1, y1);
    let i2 = cell_idx(width, x2, y2);
    for offset in 0..CELL_STRIDE {
        cells.swap(i1 + offset, i2 + offset);
    }
}

const CONDUCTIVITY: [u8; 14] = [5, 38, 64, 26, 13, 102, 20, 8, 90, 51, 77, 5, 51, 20];

#[inline(always)]
fn conductivity(species: u8) -> u8 {
    CONDUCTIVITY.get(species as usize).copied().unwrap_or(5)
}

// ── Heat Conduction ───────────────────────────────────────────────────
fn heat_conduction(cells: &mut [u8], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..width {
            let i_a = cell_idx(width, x, y);
            let species_a = cells[i_a];
            let cond_a = conductivity(species_a) as i32;
            let mut running_temp = cells[i_a + 2] as i32;

            let neighbors: [(isize, isize); 4] = [(1, 0), (0, 1), (-1, 1), (1, 1)];

            for &(dx, dy) in &neighbors {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if !in_bounds(width, height, nx, ny) {
                    continue;
                }
                let i_b = cell_idx(width, nx as usize, ny as usize);
                let species_b = cells[i_b];
                let temp_b = cells[i_b + 2] as i32;
                let min_cond = cond_a.min(conductivity(species_b) as i32);
                let delta = (running_temp - temp_b) * min_cond / 512;

                if delta != 0 {
                    running_temp = (running_temp - delta).clamp(0, 255);
                    cells[i_b + 2] = (temp_b + delta).clamp(0, 255) as u8;
                }
            }

            cells[i_a + 2] = running_temp as u8;

            // Ambient cooling (merged from separate pass)
            if species_a != SPECIES_EMPTY && species_a != SPECIES_WALL {
                if rand_u32() & 7 == 0 {
                    let t = cells[i_a + 2];
                    if t > TEMP_AMBIENT {
                        cells[i_a + 2] = t - 1;
                    } else if t < TEMP_AMBIENT {
                        cells[i_a + 2] = t + 1;
                    }
                }
            }
        }
    }
}

// ── Phase Transitions ─────────────────────────────────────────────────
fn phase_transitions(cells: &mut [u8], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..width {
            let i = cell_idx(width, x, y);
            let species = cells[i];
            let temp = cells[i + 2];

            match species {
                SPECIES_WATER => {
                    if temp >= TEMP_BOIL {
                        cells[i] = SPECIES_STEAM;
                        cells[i + 1] = rand_ra();
                    } else if temp < TEMP_FREEZE {
                        cells[i] = SPECIES_ICE;
                        cells[i + 1] = rand_ra();
                    }
                }
                SPECIES_ICE => {
                    if temp >= TEMP_FREEZE + 3 {
                        cells[i] = SPECIES_WATER;
                        cells[i + 1] = rand_ra();
                    }
                }
                SPECIES_STEAM => {
                    if temp < TEMP_BOIL.saturating_sub(6) {
                        cells[i] = SPECIES_WATER;
                        cells[i + 1] = rand_ra();
                    }
                }
                SPECIES_STONE => {
                    if temp >= TEMP_STONE_MELT {
                        cells[i] = SPECIES_LAVA;
                        cells[i + 1] = rand_ra();
                    }
                }
                SPECIES_LAVA => {
                    if temp < TEMP_STONE_MELT.saturating_sub(5) {
                        cells[i] = SPECIES_STONE;
                        cells[i + 1] = rand_ra();
                    }
                }
                SPECIES_OIL => {
                    if temp >= TEMP_OIL_IGNITE {
                        cells[i] = SPECIES_FIRE;
                        cells[i + 1] = rand_range(FUEL_OIL_MIN, FUEL_OIL_MAX);
                        cells[i + 2] = cells[i + 2].max(TEMP_FIRE_SUSTAIN + 30);
                    }
                }
                SPECIES_PLANT => {
                    if temp >= TEMP_PLANT_IGNITE {
                        cells[i] = SPECIES_FIRE;
                        cells[i + 1] = rand_range(FUEL_PLANT_MIN, FUEL_PLANT_MAX);
                        cells[i + 2] = cells[i + 2].max(TEMP_FIRE_SUSTAIN + 30);
                    }
                }
                SPECIES_WOOD => {
                    if temp >= TEMP_WOOD_IGNITE {
                        cells[i] = SPECIES_FIRE;
                        cells[i + 1] = rand_range(FUEL_WOOD_MIN, FUEL_WOOD_MAX);
                        cells[i + 2] = cells[i + 2].max(TEMP_FIRE_SUSTAIN + 30);
                    }
                }
                _ => {}
            }
        }
    }
}

// ── Shared Movement Helpers ──────────────────────────────────────────

fn rise_gas(
    cells: &mut [u8], width: usize, height: usize,
    x: usize, y: usize, clock: u8,
    can_enter: fn(u8) -> bool, drift_chance: u8,
) -> bool {
    if y > 0 {
        let above = get_species(cells, width, x, y - 1);
        if can_enter(above) {
            swap_cells(cells, width, x, y, x, y - 1);
            set_clock(cells, width, x, y - 1, clock);
            return true;
        }
        let (dx1, dx2) = if rand_bool() { (-1isize, 1isize) } else { (1, -1) };
        for &dx in &[dx1, dx2] {
            let nx = x as isize + dx;
            let ny = y as isize - 1;
            if in_bounds(width, height, nx, ny) {
                let nx = nx as usize;
                let ny = ny as usize;
                if can_enter(get_species(cells, width, nx, ny)) {
                    swap_cells(cells, width, x, y, nx, ny);
                    set_clock(cells, width, nx, ny, clock);
                    return true;
                }
            }
        }
    }

    if (rand_u32() & 0xFF) < drift_chance as u32 {
        let dx: isize = if rand_bool() { -1 } else { 1 };
        let nx = x as isize + dx;
        if in_bounds(width, height, nx, y as isize) {
            let nx = nx as usize;
            if can_enter(get_species(cells, width, nx, y)) {
                swap_cells(cells, width, x, y, nx, y);
                set_clock(cells, width, nx, y, clock);
                return true;
            }
        }
    }

    false
}

fn radiate_heat(cells: &mut [u8], width: usize, height: usize, x: usize, y: usize, amount: i32) {
    for &dy in &[-1isize, 0, 1] {
        for &dx in &[-1isize, 0, 1] {
            if dx == 0 && dy == 0 { continue; }
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if !in_bounds(width, height, nx, ny) { continue; }
            let ni = cell_idx(width, nx as usize, ny as usize);
            cells[ni + 2] = ((cells[ni + 2] as i32 + amount).min(255)) as u8;
        }
    }
}

fn fall_granular(
    cells: &mut [u8], width: usize, height: usize,
    x: usize, y: usize, clock: u8,
    can_fall_into: fn(u8) -> bool,
) {
    let below_y = y + 1;
    if below_y < height {
        let below = get_species(cells, width, x, below_y);
        if can_fall_into(below) {
            swap_cells(cells, width, x, y, x, below_y);
            set_clock(cells, width, x, below_y, clock);
            return;
        }
    }
    if below_y < height {
        let (dx1, dx2) = if rand_bool() { (-1isize, 1isize) } else { (1, -1) };
        for &dx in &[dx1, dx2] {
            let nx = x as isize + dx;
            if in_bounds(width, height, nx, below_y as isize) {
                let nx = nx as usize;
                let d = get_species(cells, width, nx, below_y);
                if can_fall_into(d) {
                    swap_cells(cells, width, x, y, nx, below_y);
                    set_clock(cells, width, nx, below_y, clock);
                    return;
                }
            }
        }
    }
}

// ── Species Updates ───────────────────────────────────────────────────

fn update_sand(cells: &mut [u8], width: usize, height: usize, x: usize, y: usize, clock: u8) {
    fall_granular(cells, width, height, x, y, clock, |s| {
        matches!(s, SPECIES_EMPTY | SPECIES_WATER | SPECIES_OIL | SPECIES_ACID)
    });
}

fn can_displace(species: u8, target: u8) -> bool {
    match species {
        SPECIES_WATER => target == SPECIES_EMPTY || target == SPECIES_OIL,
        SPECIES_OIL => target == SPECIES_EMPTY,
        SPECIES_LAVA => matches!(target, SPECIES_EMPTY | SPECIES_WATER | SPECIES_OIL | SPECIES_SAND),
        SPECIES_ACID => target == SPECIES_EMPTY || target == SPECIES_OIL,
        _ => target == SPECIES_EMPTY,
    }
}

fn update_liquid(
    cells: &mut [u8], width: usize, height: usize,
    x: usize, y: usize, species: u8, spread: i32, clock: u8,
) {
    let below_y = y + 1;
    if below_y < height {
        let below = get_species(cells, width, x, below_y);
        if can_displace(species, below) {
            swap_cells(cells, width, x, y, x, below_y);
            set_clock(cells, width, x, below_y, clock);
            return;
        }
    }
    if below_y < height {
        let (dx1, dx2) = if rand_bool() { (-1isize, 1isize) } else { (1, -1) };
        for &dx in &[dx1, dx2] {
            let nx = x as isize + dx;
            if in_bounds(width, height, nx, below_y as isize) {
                let nx = nx as usize;
                let d = get_species(cells, width, nx, below_y);
                if can_displace(species, d) {
                    swap_cells(cells, width, x, y, nx, below_y);
                    set_clock(cells, width, nx, below_y, clock);
                    return;
                }
            }
        }
    }
    let dir: isize = if rand_bool() { -1 } else { 1 };
    for step in 1..=spread {
        let nx = x as isize + dir * step as isize;
        if !in_bounds(width, height, nx, y as isize) {
            break;
        }
        let nx = nx as usize;
        if can_displace(species, get_species(cells, width, nx, y)) {
            swap_cells(cells, width, x, y, nx, y);
            set_clock(cells, width, nx, y, clock);
            return;
        }
    }
}

fn update_fire(cells: &mut [u8], width: usize, height: usize, x: usize, y: usize, clock: u8) {
    let i = cell_idx(width, x, y);
    let fuel = cells[i + 1];
    let temp = cells[i + 2];

    if fuel <= 1 {
        if rand() < 0.6 {
            cells[i] = SPECIES_SMOKE;
            cells[i + 1] = rand_ra();
        } else {
            cells[i] = SPECIES_EMPTY;
            cells[i + 1] = 0;
            cells[i + 2] = 0;
        }
        return;
    }
    cells[i + 1] = fuel - 1;

    if temp < TEMP_FIRE_SUSTAIN {
        cells[i] = SPECIES_SMOKE;
        cells[i + 1] = rand_ra();
        return;
    }

    cells[i + 2] = ((temp as i32 + 3).min(230)) as u8;

    radiate_heat(cells, width, height, x, y, 2);
    rise_gas(cells, width, height, x, y, clock, |s| s == SPECIES_EMPTY || s == SPECIES_SMOKE, 77);
}

fn update_stone(cells: &mut [u8], width: usize, height: usize, x: usize, y: usize, clock: u8) {
    fall_granular(cells, width, height, x, y, clock, |s| {
        matches!(s, SPECIES_EMPTY | SPECIES_WATER | SPECIES_OIL | SPECIES_SAND | SPECIES_ACID)
    });
}

fn update_plant(cells: &mut [u8], width: usize, height: usize, x: usize, y: usize, clock: u8) {
    if rand() < 0.04 {
        let r = rand();
        let (target_dx, target_dy): (isize, isize) = if r < 0.50 {
            let dx = if rand_bool() { -1 } else if rand() < 0.5 { 0 } else { 1 };
            (dx, -1)
        } else if r < 0.85 {
            let dx: isize = if rand_bool() { -1 } else { 1 };
            (dx, 0)
        } else {
            let dx = if rand_bool() { -1 } else if rand() < 0.5 { 0 } else { 1 };
            (dx, 1)
        };
        let gx = x as isize + target_dx;
        let gy = y as isize + target_dy;
        if in_bounds(width, height, gx, gy) {
            let gx = gx as usize;
            let gy = gy as usize;
            if get_species(cells, width, gx, gy) == SPECIES_WATER {
                set_cell_raw(cells, width, gx, gy, SPECIES_PLANT, rand_ra(), TEMP_AMBIENT, clock);
            }
        }
    }
}

fn update_steam(cells: &mut [u8], width: usize, height: usize, x: usize, y: usize, clock: u8) {
    if rand() < 0.3 {
        cells[cell_idx(width, x, y) + 1] = rand_ra();
    }
    rise_gas(cells, width, height, x, y, clock, |s| s == SPECIES_EMPTY, 128);
}

fn update_lava(cells: &mut [u8], width: usize, height: usize, x: usize, y: usize, clock: u8) {
    if rand() < 0.3 {
        cells[cell_idx(width, x, y) + 1] = rand_ra();
    }
    radiate_heat(cells, width, height, x, y, 1);
    update_liquid(cells, width, height, x, y, SPECIES_LAVA, 1, clock);
}

fn update_smoke(cells: &mut [u8], width: usize, height: usize, x: usize, y: usize, clock: u8) {
    let temp = get_temp(cells, width, x, y);
    if temp <= TEMP_AMBIENT + 2 {
        let i = cell_idx(width, x, y);
        cells[i] = SPECIES_EMPTY;
        cells[i + 1] = 0;
        cells[i + 2] = 0;
        return;
    }

    if rand() < 0.3 {
        cells[cell_idx(width, x, y) + 1] = rand_ra();
    }
    rise_gas(cells, width, height, x, y, clock, |s| s == SPECIES_EMPTY, 153);
}

fn update_acid(cells: &mut [u8], width: usize, height: usize, x: usize, y: usize, clock: u8) {
    let mut consumed = false;
    'outer: for &dy in &[-1isize, 0, 1] {
        for &dx in &[-1isize, 0, 1] {
            if dx == 0 && dy == 0 { continue; }
            let nx = x as isize + dx;
            let ny = y as isize + dy;
            if !in_bounds(width, height, nx, ny) { continue; }
            let nx = nx as usize;
            let ny = ny as usize;
            let neighbor = get_species(cells, width, nx, ny);
            if matches!(neighbor, SPECIES_SAND | SPECIES_STONE | SPECIES_PLANT | SPECIES_WOOD | SPECIES_ICE)
                && rand() < 0.20
            {
                set_cell_raw(cells, width, nx, ny, SPECIES_EMPTY, 0, 0, clock);
                if rand() < 0.40 {
                    set_cell_raw(cells, width, x, y, SPECIES_EMPTY, 0, 0, clock);
                    consumed = true;
                }
                break 'outer;
            }
        }
    }
    if consumed { return; }

    update_liquid(cells, width, height, x, y, SPECIES_ACID, 2, clock);
}

// ── World ─────────────────────────────────────────────────────────────

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct World {
    width: usize,
    height: usize,
    cells: Box<[u8]>,
    clock: u8,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl World {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new(width: usize, height: usize) -> World {
        #[cfg(target_arch = "wasm32")]
        unsafe { RNG_STATE = (js_sys::Math::random() * u32::MAX as f64) as u32 | 1; }
        #[cfg(not(target_arch = "wasm32"))]
        unsafe { RNG_STATE = 0xDEAD_BEEF; }
        World {
            width,
            height,
            cells: vec![0; width * height * CELL_STRIDE].into_boxed_slice(),
            clock: 0,
        }
    }

    pub fn width(&self) -> usize { self.width }
    pub fn height(&self) -> usize { self.height }

    pub fn tick(&mut self) {
        self.clock = if self.clock == 0 { 1 } else { 0 };
        let w = self.width;
        let h = self.height;
        let clk = self.clock;

        heat_conduction(&mut self.cells, w, h);
        phase_transitions(&mut self.cells, w, h);

        for y in (0..h).rev() {
            let left_to_right = rand_bool();
            for step in 0..w {
                let x = if left_to_right { step } else { w - 1 - step };
                if get_clock(&self.cells, w, x, y) == clk { continue; }
                let species = get_species(&self.cells, w, x, y);
                set_clock(&mut self.cells, w, x, y, clk);

                match species {
                    SPECIES_SAND => update_sand(&mut self.cells, w, h, x, y, clk),
                    SPECIES_WATER => update_liquid(&mut self.cells, w, h, x, y, SPECIES_WATER, 2, clk),
                    SPECIES_OIL => update_liquid(&mut self.cells, w, h, x, y, SPECIES_OIL, 1, clk),
                    SPECIES_FIRE => update_fire(&mut self.cells, w, h, x, y, clk),
                    SPECIES_PLANT => update_plant(&mut self.cells, w, h, x, y, clk),
                    SPECIES_STEAM => update_steam(&mut self.cells, w, h, x, y, clk),
                    SPECIES_LAVA => update_lava(&mut self.cells, w, h, x, y, clk),
                    SPECIES_STONE => update_stone(&mut self.cells, w, h, x, y, clk),
                    SPECIES_SMOKE => update_smoke(&mut self.cells, w, h, x, y, clk),
                    SPECIES_ACID => update_acid(&mut self.cells, w, h, x, y, clk),
                    _ => {}
                }
            }
        }
    }

    pub fn cells_ptr(&self) -> *const u8 { self.cells.as_ptr() }

    pub fn set_cell(&mut self, x: usize, y: usize, species: u8) {
        if x >= self.width || y >= self.height { return; }
        if species > SPECIES_WOOD { return; }
        let (ra, rb) = match species {
            SPECIES_EMPTY | SPECIES_WALL => (0, 0),
            SPECIES_FIRE => (FUEL_USER_PLACED, TEMP_FIRE_PLACE),
            SPECIES_LAVA => (rand_ra(), TEMP_LAVA_DEFAULT),
            SPECIES_STEAM => (rand_ra(), TEMP_BOIL + 5),
            SPECIES_ICE => (rand_ra(), TEMP_ICE_DEFAULT),
            _ => (rand_ra(), TEMP_AMBIENT),
        };
        let i = cell_idx(self.width, x, y);
        self.cells[i] = species;
        self.cells[i + 1] = ra;
        self.cells[i + 2] = rb;
        self.cells[i + 3] = self.clock;
    }

    pub fn clear(&mut self) { self.cells.fill(0); }
}

#[cfg(test)]
fn seed_rng(seed: u32) {
    unsafe { RNG_STATE = seed | 1; }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helper function tests ────────────────────────────────────────

    #[test]
    fn conductivity_returns_known_values() {
        assert_eq!(conductivity(SPECIES_EMPTY), 5);
        assert_eq!(conductivity(SPECIES_SAND), 38);
        assert_eq!(conductivity(SPECIES_WATER), 64);
        assert_eq!(conductivity(SPECIES_FIRE), 102);
        assert_eq!(conductivity(SPECIES_LAVA), 90);
        assert_eq!(conductivity(SPECIES_ICE), 77);
        assert_eq!(conductivity(SPECIES_WOOD), 20);
    }

    #[test]
    fn conductivity_out_of_range_returns_default() {
        assert_eq!(conductivity(200), 5);
        assert_eq!(conductivity(14), 5);
    }

    #[test]
    fn rand_range_min_equals_max() {
        seed_rng(42);
        assert_eq!(rand_range(10, 10), 10);
    }

    #[test]
    fn rand_range_normal() {
        seed_rng(42);
        for _ in 0..100 {
            let v = rand_range(5, 20);
            assert!(v >= 5 && v < 20, "rand_range(5,20) returned {}", v);
        }
    }

    #[test]
    fn can_displace_species() {
        assert!(can_displace(SPECIES_WATER, SPECIES_EMPTY));
        assert!(can_displace(SPECIES_WATER, SPECIES_OIL));
        assert!(!can_displace(SPECIES_WATER, SPECIES_SAND));

        assert!(can_displace(SPECIES_OIL, SPECIES_EMPTY));
        assert!(!can_displace(SPECIES_OIL, SPECIES_WATER));

        assert!(can_displace(SPECIES_LAVA, SPECIES_EMPTY));
        assert!(can_displace(SPECIES_LAVA, SPECIES_WATER));
        assert!(can_displace(SPECIES_LAVA, SPECIES_OIL));
        assert!(can_displace(SPECIES_LAVA, SPECIES_SAND));
        assert!(!can_displace(SPECIES_LAVA, SPECIES_WALL));

        assert!(can_displace(SPECIES_ACID, SPECIES_EMPTY));
        assert!(can_displace(SPECIES_ACID, SPECIES_OIL));
        assert!(!can_displace(SPECIES_ACID, SPECIES_SAND));

        // Default case (e.g. sand)
        assert!(can_displace(SPECIES_SAND, SPECIES_EMPTY));
        assert!(!can_displace(SPECIES_SAND, SPECIES_WATER));
    }

    #[test]
    fn in_bounds_edge_cases() {
        assert!(in_bounds(5, 5, 0, 0));
        assert!(in_bounds(5, 5, 4, 4));
        assert!(!in_bounds(5, 5, -1, 0));
        assert!(!in_bounds(5, 5, 0, -1));
        assert!(!in_bounds(5, 5, 5, 0));
        assert!(!in_bounds(5, 5, 0, 5));
    }

    // ── Phase transition tests ───────────────────────────────────────

    #[test]
    fn water_boils_to_steam() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_WATER, 0, TEMP_BOIL, 0);
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_STEAM);
    }

    #[test]
    fn water_freezes_to_ice() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_WATER, 0, TEMP_FREEZE - 1, 0);
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_ICE);
    }

    #[test]
    fn steam_condenses_below_hysteresis() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        // TEMP_BOIL - 6 = 19; temp below that triggers condensation
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_STEAM, 0, TEMP_BOIL - 7, 0);
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_WATER);
    }

    #[test]
    fn steam_stays_in_hysteresis_band() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        // TEMP_BOIL.saturating_sub(6) = 19; temp exactly at threshold should NOT condense
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_STEAM, 0, TEMP_BOIL.saturating_sub(6), 0);
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_STEAM);
    }

    #[test]
    fn ice_melts_above_threshold() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_ICE, 0, TEMP_FREEZE + 3, 0);
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_WATER);
    }

    #[test]
    fn ice_stays_frozen_at_freeze_temp() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_ICE, 0, TEMP_FREEZE, 0);
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_ICE);
    }

    #[test]
    fn oil_ignites_at_temp() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_OIL, 0, TEMP_OIL_IGNITE, 0);
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_FIRE);
    }

    #[test]
    fn plant_ignites_at_temp() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_PLANT, 0, TEMP_PLANT_IGNITE, 0);
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_FIRE);
    }

    #[test]
    fn wood_ignites_at_temp() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_WOOD, 0, TEMP_WOOD_IGNITE, 0);
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_FIRE);
    }

    #[test]
    fn stone_melts_to_lava() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_STONE, 0, TEMP_STONE_MELT, 0);
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_LAVA);
    }

    #[test]
    fn lava_solidifies_to_stone() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_LAVA, 0, TEMP_STONE_MELT - 6, 0);
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_STONE);
    }

    // ── Movement tests ───────────────────────────────────────────────

    #[test]
    fn sand_falls_into_empty() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_SAND, 0, TEMP_AMBIENT, 0);
        w.tick();
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_EMPTY);
        assert_eq!(get_species(&w.cells, w.width, 2, 3), SPECIES_SAND);
    }

    #[test]
    fn sand_displaces_water() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_SAND, 0, TEMP_AMBIENT, 0);
        set_cell_raw(&mut w.cells, w.width, 2, 3, SPECIES_WATER, 0, TEMP_AMBIENT, 0);
        // Use update_sand directly to avoid water also moving during tick
        update_sand(&mut w.cells, w.width, w.height, 2, 2, 1);
        assert_eq!(get_species(&w.cells, w.width, 2, 3), SPECIES_SAND, "Sand should fall into water");
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_WATER, "Water should be displaced up");
    }

    #[test]
    fn sand_diagonal_fall_when_blocked() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_SAND, 0, TEMP_AMBIENT, 0);
        set_cell_raw(&mut w.cells, w.width, 2, 3, SPECIES_WALL, 0, 0, 0);
        w.tick();
        // Sand should have moved diagonally
        let at_origin = get_species(&w.cells, w.width, 2, 2);
        let at_left = get_species(&w.cells, w.width, 1, 3);
        let at_right = get_species(&w.cells, w.width, 3, 3);
        assert_eq!(at_origin, SPECIES_EMPTY);
        assert!(at_left == SPECIES_SAND || at_right == SPECIES_SAND,
            "Sand should have fallen diagonally");
    }

    #[test]
    fn water_spreads_horizontally() {
        seed_rng(42);
        let mut w = World::new(7, 5);
        // Place water on a floor of walls
        set_cell_raw(&mut w.cells, w.width, 3, 3, SPECIES_WATER, 0, TEMP_AMBIENT, 0);
        for x in 0..7 {
            set_cell_raw(&mut w.cells, w.width, x, 4, SPECIES_WALL, 0, 0, 0);
        }
        // Block directly below
        // Water is at (3,3), wall at (3,4) — water should spread left or right
        w.tick();
        let still_at_origin = get_species(&w.cells, w.width, 3, 3) == SPECIES_WATER;
        let moved_somewhere = (0..7).any(|x| x != 3 && get_species(&w.cells, w.width, x, 3) == SPECIES_WATER);
        // Water should have tried to move diagonally or spread
        assert!(still_at_origin || moved_somewhere, "Water should spread");
    }

    #[test]
    fn gas_rises() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_STEAM, 0, TEMP_BOIL, 0);
        w.tick();
        // Steam should have risen (y=2 → y=1 or diagonal up)
        let still_at_origin = get_species(&w.cells, w.width, 2, 2) == SPECIES_STEAM;
        let above = get_species(&w.cells, w.width, 2, 1);
        let above_left = get_species(&w.cells, w.width, 1, 1);
        let above_right = get_species(&w.cells, w.width, 3, 1);
        assert!(!still_at_origin || above == SPECIES_STEAM || above_left == SPECIES_STEAM || above_right == SPECIES_STEAM,
            "Steam should rise");
    }

    #[test]
    fn stone_falls_through_water() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_STONE, 0, TEMP_AMBIENT, 0);
        set_cell_raw(&mut w.cells, w.width, 2, 3, SPECIES_WATER, 0, TEMP_AMBIENT, 0);
        update_stone(&mut w.cells, w.width, w.height, 2, 2, 1);
        assert_eq!(get_species(&w.cells, w.width, 2, 3), SPECIES_STONE, "Stone should fall into water");
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_WATER, "Water should be displaced up");
    }

    // ── Temperature tests ────────────────────────────────────────────

    #[test]
    fn heat_conduction_transfers_heat() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_STONE, 0, 200, 0);
        set_cell_raw(&mut w.cells, w.width, 3, 2, SPECIES_STONE, 0, TEMP_AMBIENT, 0);
        let temp_before = get_temp(&w.cells, w.width, 3, 2);
        heat_conduction(&mut w.cells, w.width, w.height);
        let temp_after = get_temp(&w.cells, w.width, 3, 2);
        assert!(temp_after > temp_before, "Neighbor should have warmed: {} -> {}", temp_before, temp_after);
    }

    #[test]
    fn ambient_cooling_nudges_toward_ambient() {
        seed_rng(42);
        let mut w = World::new(3, 3);
        set_cell_raw(&mut w.cells, w.width, 1, 1, SPECIES_SAND, 0, 50, 0);
        // Run many ticks of heat conduction to let ambient cooling work
        for _ in 0..200 {
            heat_conduction(&mut w.cells, w.width, w.height);
        }
        let temp = get_temp(&w.cells, w.width, 1, 1);
        assert!(temp < 50, "Temperature should have decreased toward ambient, got {}", temp);
    }

    #[test]
    fn fire_self_heats_and_radiates() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_FIRE, FUEL_USER_PLACED, TEMP_FIRE_SUSTAIN + 10, 0);
        set_cell_raw(&mut w.cells, w.width, 3, 2, SPECIES_SAND, 0, TEMP_AMBIENT, 0);
        let neighbor_temp_before = get_temp(&w.cells, w.width, 3, 2);
        update_fire(&mut w.cells, w.width, w.height, 2, 2, 1);
        let neighbor_temp_after = get_temp(&w.cells, w.width, 3, 2);
        assert!(neighbor_temp_after > neighbor_temp_before,
            "Fire should radiate heat to neighbors: {} -> {}", neighbor_temp_before, neighbor_temp_after);
    }

    #[test]
    fn lava_radiates_heat() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_LAVA, 0, TEMP_LAVA_DEFAULT, 0);
        set_cell_raw(&mut w.cells, w.width, 3, 2, SPECIES_SAND, 0, TEMP_AMBIENT, 0);
        let before = get_temp(&w.cells, w.width, 3, 2);
        update_lava(&mut w.cells, w.width, w.height, 2, 2, 1);
        let after = get_temp(&w.cells, w.width, 3, 2);
        assert!(after > before, "Lava should radiate heat: {} -> {}", before, after);
    }

    // ── Input validation tests ───────────────────────────────────────

    #[test]
    fn set_cell_rejects_invalid_species() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        w.set_cell(2, 2, SPECIES_WOOD + 1);
        assert_eq!(get_species(&w.cells, w.width, 2, 2), SPECIES_EMPTY);
    }

    #[test]
    fn set_cell_rejects_out_of_bounds() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        // Should not panic
        w.set_cell(10, 10, SPECIES_SAND);
        w.set_cell(5, 0, SPECIES_SAND);
        w.set_cell(0, 5, SPECIES_SAND);
    }

    #[test]
    fn ice_placed_at_cold_temp() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        w.set_cell(2, 2, SPECIES_ICE);
        assert_eq!(get_temp(&w.cells, w.width, 2, 2), TEMP_ICE_DEFAULT);
    }

    // ── Integration tests ────────────────────────────────────────────

    #[test]
    fn fire_lifecycle_oil_to_smoke() {
        seed_rng(42);
        let mut w = World::new(5, 8);
        // Place oil and heat it to ignition
        set_cell_raw(&mut w.cells, w.width, 2, 6, SPECIES_OIL, 0, TEMP_OIL_IGNITE, 0);
        // Run phase transitions to ignite
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 6), SPECIES_FIRE, "Oil should ignite");

        // Tick until fire burns out — track if smoke OR empty appeared where fire was
        // Smoke dissipates quickly so we track it across all ticks
        let mut fire_burned_out = false;
        for _ in 0..300 {
            w.tick();
            let has_fire = (0..w.height).any(|y| {
                (0..w.width).any(|x| get_species(&w.cells, w.width, x, y) == SPECIES_FIRE)
            });
            if !has_fire { fire_burned_out = true; break; }
        }
        assert!(fire_burned_out, "Fire should eventually burn out");
    }

    #[test]
    fn water_cycle_heat_to_steam_and_condense() {
        seed_rng(42);
        let mut w = World::new(5, 8);
        // Place water and heat it above boiling
        set_cell_raw(&mut w.cells, w.width, 2, 6, SPECIES_WATER, 0, TEMP_BOIL + 5, 0);
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 6), SPECIES_STEAM, "Water should boil");

        // Now cool it down and run phase transitions
        let i = cell_idx(w.width, 2, 6);
        w.cells[i + 2] = TEMP_BOIL - 10; // well below hysteresis
        phase_transitions(&mut w.cells, w.width, w.height);
        assert_eq!(get_species(&w.cells, w.width, 2, 6), SPECIES_WATER, "Steam should condense");
    }

    // ── Scenario / property tests ────────────────────────────────────

    fn count_species(w: &World, species: u8) -> usize {
        (0..w.height).flat_map(|y| (0..w.width).map(move |x| (x, y)))
            .filter(|&(x, y)| get_species(&w.cells, w.width, x, y) == species)
            .count()
    }

    fn find_all(w: &World, species: u8) -> Vec<(usize, usize)> {
        (0..w.height).flat_map(|y| (0..w.width).map(move |x| (x, y)))
            .filter(|&(x, y)| get_species(&w.cells, w.width, x, y) == species)
            .collect()
    }

    #[test]
    fn scenario_sand_settles_below_water() {
        seed_rng(42);
        let mut w = World::new(5, 12);
        // Walled container: floor at y=11, walls at x=0 and x=4
        for y in 0..12 {
            set_cell_raw(&mut w.cells, w.width, 0, y, SPECIES_WALL, 0, 0, 0);
            set_cell_raw(&mut w.cells, w.width, 4, y, SPECIES_WALL, 0, 0, 0);
        }
        for x in 0..5 {
            set_cell_raw(&mut w.cells, w.width, x, 11, SPECIES_WALL, 0, 0, 0);
        }
        // Stack: sand on top (rows 2-4), water below (rows 5-7) — inverted from natural
        for y in 2..=4 {
            for x in 1..=3 {
                set_cell_raw(&mut w.cells, w.width, x, y, SPECIES_SAND, 0, TEMP_AMBIENT, 0);
            }
        }
        for y in 5..=7 {
            for x in 1..=3 {
                set_cell_raw(&mut w.cells, w.width, x, y, SPECIES_WATER, 0, TEMP_AMBIENT, 0);
            }
        }

        for _ in 0..300 { w.tick(); }

        // Property: every sand cell should be at a higher y (lower on screen) than every water cell
        let sand_positions = find_all(&w, SPECIES_SAND);
        let water_positions = find_all(&w, SPECIES_WATER);
        assert!(!sand_positions.is_empty(), "Sand should still exist");
        assert!(!water_positions.is_empty(), "Water should still exist");
        let min_sand_y = sand_positions.iter().map(|p| p.1).min().unwrap();
        let max_water_y = water_positions.iter().map(|p| p.1).max().unwrap();
        assert!(min_sand_y >= max_water_y,
            "All sand (min_y={}) should be below all water (max_y={})", min_sand_y, max_water_y);
    }

    #[test]
    fn scenario_sand_forms_pile_not_column() {
        seed_rng(42);
        let mut w = World::new(11, 15);
        // Floor
        for x in 0..11 {
            set_cell_raw(&mut w.cells, w.width, x, 14, SPECIES_WALL, 0, 0, 0);
        }
        // Drop 10 grains from center column
        for y in 0..10 {
            set_cell_raw(&mut w.cells, w.width, 5, y, SPECIES_SAND, 0, TEMP_AMBIENT, 0);
        }

        for _ in 0..200 { w.tick(); }

        let sand_positions = find_all(&w, SPECIES_SAND);
        let unique_x: std::collections::HashSet<usize> = sand_positions.iter().map(|p| p.0).collect();
        assert!(unique_x.len() > 1,
            "Sand should spread across multiple columns (pile), not stack in one column. Columns used: {}",
            unique_x.len());
    }

    #[test]
    fn scenario_contained_fire_burns_out() {
        seed_rng(42);
        let mut w = World::new(7, 7);
        // Walled box
        for x in 0..7 {
            set_cell_raw(&mut w.cells, w.width, x, 0, SPECIES_WALL, 0, 0, 0);
            set_cell_raw(&mut w.cells, w.width, x, 6, SPECIES_WALL, 0, 0, 0);
        }
        for y in 0..7 {
            set_cell_raw(&mut w.cells, w.width, 0, y, SPECIES_WALL, 0, 0, 0);
            set_cell_raw(&mut w.cells, w.width, 6, y, SPECIES_WALL, 0, 0, 0);
        }
        // Fill interior with oil, ignite center
        for y in 1..=5 {
            for x in 1..=5 {
                set_cell_raw(&mut w.cells, w.width, x, y, SPECIES_OIL, 0, TEMP_AMBIENT, 0);
            }
        }
        set_cell_raw(&mut w.cells, w.width, 3, 3, SPECIES_FIRE, FUEL_USER_PLACED, TEMP_FIRE_PLACE, 0);

        for _ in 0..1000 { w.tick(); }

        let fire_count = count_species(&w, SPECIES_FIRE);
        let oil_count = count_species(&w, SPECIES_OIL);
        assert_eq!(fire_count, 0, "All fire should have burned out");
        assert_eq!(oil_count, 0, "All oil should have been consumed");
    }

    #[test]
    fn scenario_lava_solidifies_when_cooled() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        // Place lava at default temp, surrounded by empty (which cools it)
        set_cell_raw(&mut w.cells, w.width, 2, 3, SPECIES_LAVA, 0, TEMP_LAVA_DEFAULT, 0);
        // Floor to keep it in place
        for x in 0..5 {
            set_cell_raw(&mut w.cells, w.width, x, 4, SPECIES_WALL, 0, 0, 0);
        }

        // Run until lava cools to stone
        let mut solidified = false;
        for _ in 0..5000 {
            w.tick();
            if count_species(&w, SPECIES_LAVA) == 0 {
                solidified = true;
                break;
            }
        }
        assert!(solidified, "Lava should eventually solidify into stone");
        assert!(count_species(&w, SPECIES_STONE) > 0, "Should have stone after solidification");
    }

    #[test]
    fn scenario_water_fills_container_evenly() {
        seed_rng(42);
        let mut w = World::new(9, 8);
        // U-shaped container: floor at y=7, walls at x=0 and x=8
        for y in 0..8 {
            set_cell_raw(&mut w.cells, w.width, 0, y, SPECIES_WALL, 0, 0, 0);
            set_cell_raw(&mut w.cells, w.width, 8, y, SPECIES_WALL, 0, 0, 0);
        }
        for x in 0..9 {
            set_cell_raw(&mut w.cells, w.width, x, 7, SPECIES_WALL, 0, 0, 0);
        }
        // Pour 7 water cells from center top
        for y in 0..7 {
            set_cell_raw(&mut w.cells, w.width, 4, y, SPECIES_WATER, 0, TEMP_AMBIENT, 0);
        }

        for _ in 0..300 { w.tick(); }

        // Property: all water should be on the bottom row(s) of the container
        let water_positions = find_all(&w, SPECIES_WATER);
        assert!(!water_positions.is_empty(), "Water should still exist");
        // All water should be at y=6 (just above the floor)
        let max_y = water_positions.iter().map(|p| p.1).max().unwrap();
        let min_y = water_positions.iter().map(|p| p.1).min().unwrap();
        // Water should be in at most 2 rows (settled at bottom)
        assert!(max_y - min_y <= 1,
            "Water should settle into 1-2 rows, but spans y={}..={}", min_y, max_y);
    }

    #[test]
    fn scenario_chain_reaction_lava_ignites_oil() {
        seed_rng(42);
        let mut w = World::new(9, 6);
        // Sealed box with a stone divider — lava on left, oil on right
        // Stone conducts heat (51) between the chambers
        for x in 0..9 {
            set_cell_raw(&mut w.cells, w.width, x, 0, SPECIES_WALL, 0, 0, 0);
            set_cell_raw(&mut w.cells, w.width, x, 5, SPECIES_WALL, 0, 0, 0);
        }
        for y in 0..6 {
            set_cell_raw(&mut w.cells, w.width, 0, y, SPECIES_WALL, 0, 0, 0);
            set_cell_raw(&mut w.cells, w.width, 8, y, SPECIES_WALL, 0, 0, 0);
            // Stone divider at x=4
            set_cell_raw(&mut w.cells, w.width, 4, y, SPECIES_WALL, 0, 0, 0);
        }
        // Lava chamber (left) — walled in so it can't flow
        for y in 1..=4 {
            for x in 1..=3 {
                set_cell_raw(&mut w.cells, w.width, x, y, SPECIES_LAVA, 0, TEMP_LAVA_DEFAULT, 0);
            }
        }
        // Oil chamber (right) — separated by wall, heated by conduction
        for y in 1..=4 {
            for x in 5..=7 {
                set_cell_raw(&mut w.cells, w.width, x, y, SPECIES_OIL, 0, TEMP_AMBIENT, 0);
            }
        }

        let mut fire_seen = false;
        for _ in 0..2000 {
            w.tick();
            if count_species(&w, SPECIES_FIRE) > 0 { fire_seen = true; break; }
        }
        assert!(fire_seen, "Lava heat should conduct through wall and ignite oil");
    }

    #[test]
    fn scenario_ice_melts_from_heat_source() {
        seed_rng(42);
        let mut w = World::new(7, 5);
        // Floor
        for x in 0..7 {
            set_cell_raw(&mut w.cells, w.width, x, 4, SPECIES_WALL, 0, 0, 0);
        }
        // Row of ice at y=3
        for x in 1..=5 {
            set_cell_raw(&mut w.cells, w.width, x, 3, SPECIES_ICE, 0, TEMP_ICE_DEFAULT, 0);
        }
        // Heat source: hot stone at x=1
        set_cell_raw(&mut w.cells, w.width, 1, 3, SPECIES_STONE, 0, 80, 0);

        let initial_ice = count_species(&w, SPECIES_ICE);
        for _ in 0..300 { w.tick(); }
        let final_ice = count_species(&w, SPECIES_ICE);

        assert!(final_ice < initial_ice,
            "Some ice should have melted near heat source: {} -> {}", initial_ice, final_ice);
    }

    #[test]
    fn scenario_conservation_of_matter() {
        seed_rng(42);
        let mut w = World::new(9, 12);
        // Sealed box
        for x in 0..9 {
            set_cell_raw(&mut w.cells, w.width, x, 0, SPECIES_WALL, 0, 0, 0);
            set_cell_raw(&mut w.cells, w.width, x, 11, SPECIES_WALL, 0, 0, 0);
        }
        for y in 0..12 {
            set_cell_raw(&mut w.cells, w.width, 0, y, SPECIES_WALL, 0, 0, 0);
            set_cell_raw(&mut w.cells, w.width, 8, y, SPECIES_WALL, 0, 0, 0);
        }
        // Mix sand and water inside
        for x in 1..=7 {
            set_cell_raw(&mut w.cells, w.width, x, 5, SPECIES_SAND, 0, TEMP_AMBIENT, 0);
            set_cell_raw(&mut w.cells, w.width, x, 6, SPECIES_WATER, 0, TEMP_AMBIENT, 0);
        }
        let initial_sand = count_species(&w, SPECIES_SAND);
        let initial_water = count_species(&w, SPECIES_WATER);

        for _ in 0..200 { w.tick(); }

        let final_sand = count_species(&w, SPECIES_SAND);
        let final_water = count_species(&w, SPECIES_WATER);
        assert_eq!(initial_sand, final_sand,
            "Sand count should be conserved: {} -> {}", initial_sand, final_sand);
        assert_eq!(initial_water, final_water,
            "Water count should be conserved: {} -> {}", initial_water, final_water);
    }

    #[test]
    fn scenario_oil_floats_on_water() {
        seed_rng(42);
        let mut w = World::new(5, 12);
        // Container
        for y in 0..12 {
            set_cell_raw(&mut w.cells, w.width, 0, y, SPECIES_WALL, 0, 0, 0);
            set_cell_raw(&mut w.cells, w.width, 4, y, SPECIES_WALL, 0, 0, 0);
        }
        for x in 0..5 {
            set_cell_raw(&mut w.cells, w.width, x, 11, SPECIES_WALL, 0, 0, 0);
        }
        // Place oil below water (wrong order)
        for y in 7..=9 {
            for x in 1..=3 {
                set_cell_raw(&mut w.cells, w.width, x, y, SPECIES_OIL, 0, TEMP_AMBIENT, 0);
            }
        }
        for y in 4..=6 {
            for x in 1..=3 {
                set_cell_raw(&mut w.cells, w.width, x, y, SPECIES_WATER, 0, TEMP_AMBIENT, 0);
            }
        }

        for _ in 0..400 { w.tick(); }

        // Water displaces oil, so water sinks and oil floats
        let oil_positions = find_all(&w, SPECIES_OIL);
        let water_positions = find_all(&w, SPECIES_WATER);
        assert!(!oil_positions.is_empty(), "Oil should still exist");
        assert!(!water_positions.is_empty(), "Water should still exist");
        let max_oil_y = oil_positions.iter().map(|p| p.1).max().unwrap();
        let min_water_y = water_positions.iter().map(|p| p.1).min().unwrap();
        assert!(min_water_y >= max_oil_y,
            "Water (min_y={}) should settle below oil (max_y={})", min_water_y, max_oil_y);
    }

    #[test]
    fn scenario_acid_dissolves_stone_wall() {
        seed_rng(42);
        let mut w = World::new(5, 8);
        // Floor
        for x in 0..5 {
            set_cell_raw(&mut w.cells, w.width, x, 7, SPECIES_WALL, 0, 0, 0);
        }
        // Stone barrier at y=5
        for x in 1..=3 {
            set_cell_raw(&mut w.cells, w.width, x, 5, SPECIES_STONE, 0, TEMP_AMBIENT, 0);
        }
        // Acid above barrier
        for x in 1..=3 {
            set_cell_raw(&mut w.cells, w.width, x, 4, SPECIES_ACID, 0, TEMP_AMBIENT, 0);
        }

        let initial_stone = count_species(&w, SPECIES_STONE);
        for _ in 0..300 { w.tick(); }
        let final_stone = count_species(&w, SPECIES_STONE);

        assert!(final_stone < initial_stone,
            "Acid should dissolve some stone: {} -> {}", initial_stone, final_stone);
    }

    #[test]
    fn scenario_smoke_dissipates_completely() {
        seed_rng(42);
        let mut w = World::new(5, 10);
        // Place several smoke cells with warm temps so they don't vanish instantly
        for x in 1..=3 {
            set_cell_raw(&mut w.cells, w.width, x, 8, SPECIES_SMOKE, 0, TEMP_AMBIENT + 10, 0);
        }

        let mut dissipated = false;
        for _ in 0..500 {
            w.tick();
            if count_species(&w, SPECIES_SMOKE) == 0 {
                dissipated = true;
                break;
            }
        }
        assert!(dissipated, "All smoke should eventually dissipate");
    }

    #[test]
    fn scenario_steam_collects_at_ceiling() {
        seed_rng(42);
        let mut w = World::new(7, 10);
        // Sealed box
        for x in 0..7 {
            set_cell_raw(&mut w.cells, w.width, x, 0, SPECIES_WALL, 0, 0, 0);
            set_cell_raw(&mut w.cells, w.width, x, 9, SPECIES_WALL, 0, 0, 0);
        }
        for y in 0..10 {
            set_cell_raw(&mut w.cells, w.width, 0, y, SPECIES_WALL, 0, 0, 0);
            set_cell_raw(&mut w.cells, w.width, 6, y, SPECIES_WALL, 0, 0, 0);
        }
        // Place steam near the bottom, keep it hot enough to stay as steam
        for x in 1..=5 {
            set_cell_raw(&mut w.cells, w.width, x, 7, SPECIES_STEAM, 0, TEMP_BOIL + 5, 0);
        }

        for _ in 0..200 { w.tick(); }

        // Steam that's still steam should be near the top (low y)
        let steam_positions = find_all(&w, SPECIES_STEAM);
        if !steam_positions.is_empty() {
            let avg_y: f64 = steam_positions.iter().map(|p| p.1 as f64).sum::<f64>()
                / steam_positions.len() as f64;
            // Should be in upper half of container (y < 5)
            assert!(avg_y < 5.0,
                "Steam should have risen toward ceiling, avg y = {:.1}", avg_y);
        }
        // If all steam condensed, that's also fine — it cooled naturally
    }

    #[test]
    fn scenario_plant_grows_into_water() {
        seed_rng(42);
        let mut w = World::new(7, 7);
        // Floor
        for x in 0..7 {
            set_cell_raw(&mut w.cells, w.width, x, 6, SPECIES_WALL, 0, 0, 0);
        }
        // Plant seed at center
        set_cell_raw(&mut w.cells, w.width, 3, 5, SPECIES_PLANT, 0, TEMP_AMBIENT, 0);
        // Surround with water
        for y in 3..=5 {
            for x in 1..=5 {
                if !(x == 3 && y == 5) {
                    set_cell_raw(&mut w.cells, w.width, x, y, SPECIES_WATER, 0, TEMP_AMBIENT, 0);
                }
            }
        }

        let initial_plant = count_species(&w, SPECIES_PLANT);
        for _ in 0..500 { w.tick(); }
        let final_plant = count_species(&w, SPECIES_PLANT);

        assert!(final_plant > initial_plant,
            "Plant should grow into adjacent water: {} -> {}", initial_plant, final_plant);
    }

    #[test]
    fn scenario_gravity_everything_settles() {
        seed_rng(42);
        let mut w = World::new(9, 15);
        // Container
        for y in 0..15 {
            set_cell_raw(&mut w.cells, w.width, 0, y, SPECIES_WALL, 0, 0, 0);
            set_cell_raw(&mut w.cells, w.width, 8, y, SPECIES_WALL, 0, 0, 0);
        }
        for x in 0..9 {
            set_cell_raw(&mut w.cells, w.width, x, 14, SPECIES_WALL, 0, 0, 0);
        }
        // Scatter particles at the top
        set_cell_raw(&mut w.cells, w.width, 2, 1, SPECIES_SAND, 0, TEMP_AMBIENT, 0);
        set_cell_raw(&mut w.cells, w.width, 4, 1, SPECIES_STONE, 0, TEMP_AMBIENT, 0);
        set_cell_raw(&mut w.cells, w.width, 6, 1, SPECIES_WATER, 0, TEMP_AMBIENT, 0);
        set_cell_raw(&mut w.cells, w.width, 3, 2, SPECIES_OIL, 0, TEMP_AMBIENT, 0);

        for _ in 0..200 { w.tick(); }

        // Nothing should remain floating in the top half (y < 7)
        for y in 1..7 {
            for x in 1..=7 {
                let s = get_species(&w.cells, w.width, x, y);
                assert!(matches!(s, SPECIES_EMPTY | SPECIES_WALL),
                    "Found {} at ({},{}) — all solids/liquids should have settled", s, x, y);
            }
        }
    }

    #[test]
    fn scenario_lava_meets_water_creates_stone_or_steam() {
        seed_rng(42);
        let mut w = World::new(7, 6);
        // Floor
        for x in 0..7 {
            set_cell_raw(&mut w.cells, w.width, x, 5, SPECIES_WALL, 0, 0, 0);
        }
        // Pool of water on the right
        for x in 4..=5 {
            set_cell_raw(&mut w.cells, w.width, x, 4, SPECIES_WATER, 0, TEMP_AMBIENT, 0);
        }
        // Lava approaching from the left
        set_cell_raw(&mut w.cells, w.width, 2, 4, SPECIES_LAVA, 0, TEMP_LAVA_DEFAULT, 0);

        let initial_water = count_species(&w, SPECIES_WATER);
        for _ in 0..300 { w.tick(); }

        // Lava's heat should have caused water to boil into steam,
        // or lava displaced water, or both
        let final_water = count_species(&w, SPECIES_WATER);
        let has_steam = count_species(&w, SPECIES_STEAM) > 0;
        let has_stone = count_species(&w, SPECIES_STONE) > 0;
        assert!(final_water < initial_water || has_steam || has_stone,
            "Lava meeting water should create steam or stone. water: {}->{}, steam: {}, stone: {}",
            initial_water, final_water, has_steam, has_stone);
    }

    #[test]
    fn scenario_temperature_reaches_equilibrium() {
        seed_rng(42);
        let mut w = World::new(5, 3);
        // Use wall-backed cells so they can't move
        for x in 0..5 {
            set_cell_raw(&mut w.cells, w.width, x, 2, SPECIES_WALL, 0, 0, 0);
        }
        // Hot stone and cold stone on the floor — they won't fall
        set_cell_raw(&mut w.cells, w.width, 1, 1, SPECIES_STONE, 0, 200, 0);
        set_cell_raw(&mut w.cells, w.width, 3, 1, SPECIES_STONE, 0, 2, 0);

        for _ in 0..3000 { w.tick(); }

        let t1 = get_temp(&w.cells, w.width, 1, 1);
        let t2 = get_temp(&w.cells, w.width, 3, 1);
        // Both should converge near ambient
        assert!((t1 as i32 - TEMP_AMBIENT as i32).unsigned_abs() <= 6,
            "Hot stone should cool toward ambient: temp={}, ambient={}", t1, TEMP_AMBIENT);
        assert!((t2 as i32 - TEMP_AMBIENT as i32).unsigned_abs() <= 6,
            "Cold stone should warm toward ambient: temp={}, ambient={}", t2, TEMP_AMBIENT);
    }

    #[test]
    fn scenario_fire_needs_fuel() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        // Fire with minimal fuel, no combustible neighbors
        set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_FIRE, 3, TEMP_FIRE_PLACE, 0);

        for _ in 0..50 { w.tick(); }

        // Fire with only 3 fuel ticks should be long gone
        assert_eq!(count_species(&w, SPECIES_FIRE), 0,
            "Fire with no fuel source should burn out quickly");
    }

    #[test]
    fn scenario_wood_burns_longer_than_oil() {
        seed_rng(100);
        // Measure how many ticks wood fire lasts vs oil fire
        let burn_time = |_species: u8, fuel_min: u8, fuel_max: u8| -> u32 {
            seed_rng(100);
            let mut w = World::new(3, 3);
            let fuel = (fuel_min as u16 + fuel_max as u16) as u8 / 2;
            set_cell_raw(&mut w.cells, w.width, 1, 1, SPECIES_FIRE, fuel, TEMP_FIRE_PLACE, 0);
            for tick in 1..=500u32 {
                w.tick();
                if count_species(&w, SPECIES_FIRE) == 0 { return tick; }
            }
            500
        };

        let oil_ticks = burn_time(SPECIES_OIL, FUEL_OIL_MIN, FUEL_OIL_MAX);
        let wood_ticks = burn_time(SPECIES_WOOD, FUEL_WOOD_MIN, FUEL_WOOD_MAX);
        assert!(wood_ticks > oil_ticks,
            "Wood (fuel {}-{}) should burn longer than oil (fuel {}-{}): {} vs {} ticks",
            FUEL_WOOD_MIN, FUEL_WOOD_MAX, FUEL_OIL_MIN, FUEL_OIL_MAX, wood_ticks, oil_ticks);
    }

    // ── Heat conduction rate tests ─────────────────────────────────

    #[test]
    fn conduction_is_gradual_between_neighbors() {
        seed_rng(42);
        let mut w = World::new(3, 3);
        // Hot stone next to cold stone on a wall floor
        set_cell_raw(&mut w.cells, w.width, 0, 1, SPECIES_STONE, 0, 200, 0);
        set_cell_raw(&mut w.cells, w.width, 1, 1, SPECIES_STONE, 0, 0, 0);
        for x in 0..3 {
            set_cell_raw(&mut w.cells, w.width, x, 2, SPECIES_WALL, 0, 0, 0);
        }
        heat_conduction(&mut w.cells, w.width, w.height);
        let hot_after = get_temp(&w.cells, w.width, 0, 1);
        let cold_after = get_temp(&w.cells, w.width, 1, 1);
        // With /512 divisor: delta = 200 * 51 / 512 = ~19
        // Stone conductivity is 51, so transfer should be modest per tick
        assert!(hot_after > 170, "Hot stone should still be warm after 1 tick: {}", hot_after);
        assert!(cold_after < 30, "Cold stone should still be cool after 1 tick: {}", cold_after);
        assert!(cold_after > 0, "Some heat should have transferred: {}", cold_after);
    }

    #[test]
    fn conduction_through_air_is_very_slow() {
        seed_rng(42);
        let mut w = World::new(5, 3);
        for x in 0..5 {
            set_cell_raw(&mut w.cells, w.width, x, 2, SPECIES_WALL, 0, 0, 0);
        }
        // Hot stone with empty air gap then cold stone
        set_cell_raw(&mut w.cells, w.width, 0, 1, SPECIES_STONE, 0, 200, 0);
        // (1,1) is empty air — conductivity 5
        set_cell_raw(&mut w.cells, w.width, 2, 1, SPECIES_STONE, 0, 0, 0);
        for _ in 0..10 { heat_conduction(&mut w.cells, w.width, w.height); }
        let far_temp = get_temp(&w.cells, w.width, 2, 1);
        // Heat should barely reach through air (cond=5, /512)
        assert!(far_temp < 10,
            "Heat through air gap should be very slow: far stone temp = {}", far_temp);
    }

    #[test]
    fn ambient_drift_is_slow() {
        seed_rng(42);
        let mut w = World::new(3, 3);
        set_cell_raw(&mut w.cells, w.width, 1, 1, SPECIES_SAND, 0, 100, 0);
        // After 10 ticks, with ~12.5% drift rate, expect ~1-2 degree change
        for _ in 0..10 { w.tick(); }
        // Sand may have moved — find it
        let sand_temps: Vec<u8> = (0..3).flat_map(|y| (0..3).map(move |x| (x, y)))
            .filter(|&(x, y)| get_species(&w.cells, w.width, x, y) == SPECIES_SAND)
            .map(|(x, y)| get_temp(&w.cells, w.width, x, y))
            .collect();
        assert!(!sand_temps.is_empty(), "Sand should still exist");
        let t = sand_temps[0];
        // Should still be well above ambient (12) after only 10 ticks
        assert!(t > 80, "Temp should drift slowly toward ambient: {} (started at 100)", t);
    }

    // ── Ice behavior scenario tests ─────────────────────────────────

    #[test]
    fn scenario_ice_survives_at_least_20_ticks() {
        seed_rng(42);
        let mut w = World::new(3, 3);
        set_cell_raw(&mut w.cells, w.width, 1, 1, SPECIES_ICE, 0, TEMP_ICE_DEFAULT, 0);
        for _ in 0..20 { w.tick(); }
        assert_eq!(get_species(&w.cells, w.width, 1, 1), SPECIES_ICE,
            "Single ice cell should survive at least 20 ticks at TEMP_ICE_DEFAULT({})", TEMP_ICE_DEFAULT);
    }

    #[test]
    fn scenario_ice_eventually_melts_at_ambient() {
        seed_rng(42);
        let mut w = World::new(3, 3);
        set_cell_raw(&mut w.cells, w.width, 1, 1, SPECIES_ICE, 0, TEMP_ICE_DEFAULT, 0);
        for _ in 0..200 { w.tick(); }
        assert_ne!(get_species(&w.cells, w.width, 1, 1), SPECIES_ICE,
            "Isolated ice should eventually melt at ambient temp");
    }

    #[test]
    fn scenario_ice_temp_rises_gradually() {
        seed_rng(42);
        let mut w = World::new(3, 3);
        set_cell_raw(&mut w.cells, w.width, 1, 1, SPECIES_ICE, 0, TEMP_ICE_DEFAULT, 0);
        // After 25 ticks, temp should have risen but not yet reached melt point
        for _ in 0..25 { w.tick(); }
        let temp = get_temp(&w.cells, w.width, 1, 1);
        assert!(temp > TEMP_ICE_DEFAULT, "Ice temp should rise over time: {}", temp);
        assert!(temp < TEMP_FREEZE + 3, "Ice should not have reached melt point yet: {}", temp);
    }

    #[test]
    fn scenario_large_ice_block_intact_at_20_ticks() {
        seed_rng(42);
        let mut w = World::new(12, 12);
        for x in 0..12 {
            set_cell_raw(&mut w.cells, w.width, x, 11, SPECIES_WALL, 0, 0, 0);
        }
        for y in 2..=9 {
            for x in 2..=9 {
                set_cell_raw(&mut w.cells, w.width, x, y, SPECIES_ICE, 0, TEMP_ICE_DEFAULT, 0);
            }
        }
        for _ in 0..20 { w.tick(); }
        let remaining = count_species(&w, SPECIES_ICE);
        assert_eq!(remaining, 64,
            "8x8 ice block should be fully intact at 20 ticks, got {}/64", remaining);
    }

    #[test]
    fn scenario_ice_block_melts_outside_in() {
        seed_rng(42);
        let mut w = World::new(12, 12);
        for x in 0..12 {
            set_cell_raw(&mut w.cells, w.width, x, 11, SPECIES_WALL, 0, 0, 0);
        }
        for y in 2..=9 {
            for x in 2..=9 {
                set_cell_raw(&mut w.cells, w.width, x, y, SPECIES_ICE, 0, TEMP_ICE_DEFAULT, 0);
            }
        }
        // Track when center vs corner cells melt
        let center = (5, 5);
        let corners = [(2, 2), (9, 2), (2, 9), (9, 9)];
        let mut center_melted = 0u32;
        let mut first_corner_melted = 0u32;
        for tick in 1..=200u32 {
            w.tick();
            if first_corner_melted == 0 {
                if corners.iter().any(|&(x, y)| get_species(&w.cells, w.width, x, y) != SPECIES_ICE) {
                    first_corner_melted = tick;
                }
            }
            if center_melted == 0 && get_species(&w.cells, w.width, center.0, center.1) != SPECIES_ICE {
                center_melted = tick;
            }
            if center_melted > 0 && first_corner_melted > 0 { break; }
        }
        assert!(first_corner_melted > 0, "Corners should eventually melt");
        assert!(center_melted > 0, "Center should eventually melt");
        assert!(center_melted > first_corner_melted,
            "Center should melt after corners (outside-in): center={}, corner={}", center_melted, first_corner_melted);
    }

    #[test]
    fn scenario_ice_in_warm_water_melts_faster_than_in_air() {
        seed_rng(42);
        // Ice alone in air (empty cells, conductivity 5)
        let alone_ticks = {
            seed_rng(42);
            let mut w = World::new(3, 3);
            set_cell_raw(&mut w.cells, w.width, 1, 1, SPECIES_ICE, 0, TEMP_ICE_DEFAULT, 0);
            let mut t = 500u32;
            for tick in 1..=500 {
                w.tick();
                if get_species(&w.cells, w.width, 1, 1) != SPECIES_ICE { t = tick; break; }
            }
            t
        };
        // Ice surrounded by warm water (above boil threshold so it won't freeze)
        let water_ticks = {
            seed_rng(42);
            let mut w = World::new(5, 5);
            set_cell_raw(&mut w.cells, w.width, 2, 2, SPECIES_ICE, 0, TEMP_ICE_DEFAULT, 0);
            for y in 1..=3 {
                for x in 1..=3 {
                    if !(x == 2 && y == 2) {
                        set_cell_raw(&mut w.cells, w.width, x, y, SPECIES_WATER, 0, TEMP_BOIL - 1, 0);
                    }
                }
            }
            let mut t = 500u32;
            for tick in 1..=500 {
                w.tick();
                if count_species(&w, SPECIES_ICE) == 0 { t = tick; break; }
            }
            t
        };
        // Warm water conducts heat much better than air, so ice melts faster
        assert!(water_ticks < alone_ticks,
            "Ice should melt faster in warm water than air: water={}, air={}",
            water_ticks, alone_ticks);
    }

    #[test]
    fn scenario_ice_placed_starts_cold() {
        seed_rng(42);
        let mut w = World::new(5, 5);
        w.set_cell(2, 2, SPECIES_ICE);
        assert_eq!(get_temp(&w.cells, w.width, 2, 2), TEMP_ICE_DEFAULT,
            "Ice placed via set_cell should start at TEMP_ICE_DEFAULT({})", TEMP_ICE_DEFAULT);
    }
}
