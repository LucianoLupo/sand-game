use wasm_bindgen::prelude::*;

const SPECIES_EMPTY: u8 = 0;
const SPECIES_SAND: u8 = 1;
const SPECIES_WATER: u8 = 2;
const SPECIES_OIL: u8 = 3;
const SPECIES_WALL: u8 = 4;

const CELL_STRIDE: usize = 4;

fn rand() -> f64 {
    js_sys::Math::random()
}

fn rand_bool() -> bool {
    rand() < 0.5
}

fn rand_ra() -> u8 {
    (rand() * 30.0) as u8
}

#[inline(always)]
fn cell_idx(width: usize, x: usize, y: usize) -> usize {
    (y * width + x) * CELL_STRIDE
}

fn in_bounds(width: usize, height: usize, x: isize, y: isize) -> bool {
    x >= 0 && y >= 0 && (x as usize) < width && (y as usize) < height
}

fn set_clock(cells: &mut [u8], width: usize, x: usize, y: usize, clock: u8) {
    let i = cell_idx(width, x, y);
    cells[i + 3] = clock;
}

fn get_species(cells: &[u8], width: usize, x: usize, y: usize) -> u8 {
    cells[cell_idx(width, x, y)]
}

fn get_clock(cells: &[u8], width: usize, x: usize, y: usize) -> u8 {
    cells[cell_idx(width, x, y) + 3]
}

fn swap_cells(cells: &mut [u8], width: usize, x1: usize, y1: usize, x2: usize, y2: usize) {
    let i1 = cell_idx(width, x1, y1);
    let i2 = cell_idx(width, x2, y2);
    for offset in 0..CELL_STRIDE {
        cells.swap(i1 + offset, i2 + offset);
    }
}

fn update_sand(cells: &mut [u8], width: usize, height: usize, x: usize, y: usize, clock: u8) {
    let below_y = y + 1;

    if below_y < height {
        let below_species = get_species(cells, width, x, below_y);
        if below_species == SPECIES_EMPTY
            || below_species == SPECIES_WATER
            || below_species == SPECIES_OIL
        {
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
                let diag_species = get_species(cells, width, nx, below_y);
                if diag_species == SPECIES_EMPTY
                    || diag_species == SPECIES_WATER
                    || diag_species == SPECIES_OIL
                {
                    swap_cells(cells, width, x, y, nx, below_y);
                    set_clock(cells, width, nx, below_y, clock);
                    return;
                }
            }
        }
    }
}

fn update_liquid(
    cells: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    species: u8,
    spread: i32,
    clock: u8,
) {
    let below_y = y + 1;
    let is_water = species == SPECIES_WATER;

    if below_y < height {
        let below_species = get_species(cells, width, x, below_y);
        if below_species == SPECIES_EMPTY {
            swap_cells(cells, width, x, y, x, below_y);
            set_clock(cells, width, x, below_y, clock);
            return;
        }
        if is_water && below_species == SPECIES_OIL {
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
                let diag_species = get_species(cells, width, nx, below_y);
                if diag_species == SPECIES_EMPTY || (is_water && diag_species == SPECIES_OIL) {
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
        let target_species = get_species(cells, width, nx, y);
        if target_species == SPECIES_EMPTY {
            swap_cells(cells, width, x, y, nx, y);
            set_clock(cells, width, nx, y, clock);
            return;
        }
        if is_water && target_species == SPECIES_OIL {
            swap_cells(cells, width, x, y, nx, y);
            set_clock(cells, width, nx, y, clock);
            return;
        }
        break;
    }
}

#[wasm_bindgen]
pub struct World {
    width: usize,
    height: usize,
    cells: Vec<u8>,
    clock: u8,
}

#[wasm_bindgen]
impl World {
    #[wasm_bindgen(constructor)]
    pub fn new(width: usize, height: usize) -> World {
        World {
            width,
            height,
            cells: vec![0; width * height * CELL_STRIDE],
            clock: 0,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn tick(&mut self) {
        self.clock = if self.clock == 0 { 1 } else { 0 };
        let w = self.width;
        let h = self.height;
        let clk = self.clock;

        for y in (0..h).rev() {
            let left_to_right = rand_bool();
            for step in 0..w {
                let x = if left_to_right { step } else { w - 1 - step };

                if get_clock(&self.cells, w, x, y) == clk {
                    continue;
                }

                let species = get_species(&self.cells, w, x, y);
                set_clock(&mut self.cells, w, x, y, clk);

                match species {
                    SPECIES_SAND => {
                        update_sand(&mut self.cells, w, h, x, y, clk);
                    }
                    SPECIES_WATER => {
                        update_liquid(&mut self.cells, w, h, x, y, SPECIES_WATER, 2, clk);
                    }
                    SPECIES_OIL => {
                        update_liquid(&mut self.cells, w, h, x, y, SPECIES_OIL, 1, clk);
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn cells_ptr(&self) -> *const u8 {
        self.cells.as_ptr()
    }

    pub fn set_cell(&mut self, x: usize, y: usize, species: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let ra = if species != SPECIES_EMPTY && species != SPECIES_WALL {
            rand_ra()
        } else {
            0
        };
        let i = cell_idx(self.width, x, y);
        self.cells[i] = species;
        self.cells[i + 1] = ra;
        self.cells[i + 2] = 0;
        self.cells[i + 3] = self.clock;
    }

    pub fn clear(&mut self) {
        self.cells.fill(0);
    }
}
