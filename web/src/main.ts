import init, { World } from "./wasm/sand_sim";
import { Renderer } from "./renderer";
import { UI } from "./ui";
import { Species } from "./types";

const SIM_WIDTH = 300;
const SIM_HEIGHT = 200;
const TICKS_PER_FRAME = 3;

async function main() {
  const wasm = await init();
  const world = new World(SIM_WIDTH, SIM_HEIGHT);
  const memory = wasm.memory;

  const canvas = document.getElementById("sand-canvas") as HTMLCanvasElement;
  const renderer = new Renderer(canvas, SIM_WIDTH, SIM_HEIGHT);

  function drawBrush(cx: number, cy: number, species: Species, size: number) {
    const r = Math.max(0, size - 1);
    for (let dy = -r; dy <= r; dy++) {
      for (let dx = -r; dx <= r; dx++) {
        if (dx * dx + dy * dy <= r * r + r) {
          const x = cx + dx;
          const y = cy + dy;
          if (x >= 0 && x < SIM_WIDTH && y >= 0 && y < SIM_HEIGHT) {
            world.set_cell(x, y, species);
          }
        }
      }
    }
  }

  const ui = new UI(canvas, SIM_WIDTH, SIM_HEIGHT, drawBrush, () => world.clear());

  window.addEventListener("resize", () => renderer.resize());

  let lastTime = performance.now();
  let frameCount = 0;

  function loop() {
    if (!ui.paused) {
      ui.spawnFaucets((x, y, s) => world.set_cell(x, y, s));
      for (let i = 0; i < TICKS_PER_FRAME; i++) {
        world.tick();
      }
    }

    renderer.render(world.cells_ptr(), memory);

    frameCount++;
    const now = performance.now();
    if (now - lastTime >= 1000) {
      ui.updateFPS(frameCount);
      frameCount = 0;
      lastTime = now;
    }

    requestAnimationFrame(loop);
  }

  requestAnimationFrame(loop);
}

main();
