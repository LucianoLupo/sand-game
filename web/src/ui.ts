import { Species, type Faucet } from "./types";

const ELEMENT_MAP: Record<string, Species> = {
  sand: Species.Sand,
  water: Species.Water,
  oil: Species.Oil,
  lava: Species.Lava,
  fire: Species.Fire,
  steam: Species.Steam,
  plant: Species.Plant,
  stone: Species.Stone,
  wall: Species.Wall,
  eraser: Species.Empty,
};

export class UI {
  selectedElement: Species = Species.Sand;
  brushSize: number = 3;
  faucetMode: boolean = false;
  faucets: Faucet[] = [];
  paused: boolean = false;

  private canvas: HTMLCanvasElement;
  private simWidth: number;
  private simHeight: number;
  private onDraw: (x: number, y: number, species: Species, brushSize: number) => void;
  private onClear: () => void;
  private mouseDown: boolean = false;
  private abortController: AbortController;

  constructor(
    canvas: HTMLCanvasElement,
    simWidth: number,
    simHeight: number,
    onDraw: (x: number, y: number, species: Species, brushSize: number) => void,
    onClear: () => void,
  ) {
    this.canvas = canvas;
    this.simWidth = simWidth;
    this.simHeight = simHeight;
    this.onDraw = onDraw;
    this.onClear = onClear;
    this.abortController = new AbortController();

    this.bindElementButtons();
    this.bindBrushSize();
    this.bindFaucetMode();
    this.bindPauseClear();
    this.bindCanvasMouse();
    this.bindCanvasTouch();
  }

  updateFPS(fps: number): void {
    document.getElementById("fps")!.textContent = `${fps} FPS`;
  }

  spawnFaucets(setCell: (x: number, y: number, species: number) => void): void {
    for (const f of this.faucets) {
      setCell(f.x, f.y, f.species);
    }
  }

  destroy(): void {
    this.abortController.abort();
  }

  private toSimCoords(offsetX: number, offsetY: number): { x: number; y: number } {
    const x = Math.floor((offsetX / this.canvas.clientWidth) * this.simWidth);
    const y = Math.floor((offsetY / this.canvas.clientHeight) * this.simHeight);
    return { x, y };
  }

  private handleDraw(offsetX: number, offsetY: number): void {
    const { x, y } = this.toSimCoords(offsetX, offsetY);

    if (this.faucetMode && this.selectedElement !== Species.Empty) {
      const idx = this.faucets.findIndex((f) => f.x === x && f.y === y);
      if (idx >= 0) {
        this.faucets.splice(idx, 1);
      } else {
        this.faucets.push({ x, y, species: this.selectedElement });
      }
      return;
    }

    this.onDraw(x, y, this.selectedElement, this.brushSize);
  }

  private bindElementButtons(): void {
    const signal = this.abortController.signal;
    const buttons = document.querySelectorAll<HTMLButtonElement>(".element-btn");

    for (const btn of buttons) {
      btn.addEventListener(
        "click",
        () => {
          const element = btn.dataset.element!;
          this.selectedElement = ELEMENT_MAP[element];
          for (const b of buttons) b.classList.remove("active");
          btn.classList.add("active");
        },
        { signal },
      );
    }
  }

  private bindBrushSize(): void {
    const signal = this.abortController.signal;
    const range = document.getElementById("brush-size") as HTMLInputElement;
    const label = document.getElementById("brush-label")!;

    range.addEventListener(
      "input",
      () => {
        this.brushSize = Number(range.value);
        label.textContent = range.value;
      },
      { signal },
    );
  }

  private bindFaucetMode(): void {
    const signal = this.abortController.signal;
    const checkbox = document.getElementById("faucet-mode") as HTMLInputElement;

    checkbox.addEventListener(
      "change",
      () => {
        this.faucetMode = checkbox.checked;
      },
      { signal },
    );
  }

  private bindPauseClear(): void {
    const signal = this.abortController.signal;
    const pauseBtn = document.getElementById("pause-btn")!;
    const clearBtn = document.getElementById("clear-btn")!;

    pauseBtn.addEventListener(
      "click",
      () => {
        this.paused = !this.paused;
        pauseBtn.textContent = this.paused ? "Play" : "Pause";
      },
      { signal },
    );

    clearBtn.addEventListener(
      "click",
      () => {
        this.faucets = [];
        this.onClear();
      },
      { signal },
    );
  }

  private bindCanvasMouse(): void {
    const signal = this.abortController.signal;

    this.canvas.addEventListener(
      "mousedown",
      (e: MouseEvent) => {
        this.mouseDown = true;
        this.handleDraw(e.offsetX, e.offsetY);
      },
      { signal },
    );

    this.canvas.addEventListener(
      "mousemove",
      (e: MouseEvent) => {
        if (!this.mouseDown) return;
        this.handleDraw(e.offsetX, e.offsetY);
      },
      { signal },
    );

    window.addEventListener(
      "mouseup",
      () => {
        this.mouseDown = false;
      },
      { signal },
    );

    this.canvas.addEventListener(
      "mouseleave",
      () => {
        this.mouseDown = false;
      },
      { signal },
    );
  }

  private bindCanvasTouch(): void {
    const signal = this.abortController.signal;

    const getTouchOffset = (e: TouchEvent) => {
      const touch = e.touches[0];
      const rect = this.canvas.getBoundingClientRect();
      return {
        offsetX: touch.clientX - rect.left,
        offsetY: touch.clientY - rect.top,
      };
    };

    this.canvas.addEventListener(
      "touchstart",
      (e: TouchEvent) => {
        e.preventDefault();
        this.mouseDown = true;
        const { offsetX, offsetY } = getTouchOffset(e);
        this.handleDraw(offsetX, offsetY);
      },
      { signal, passive: false },
    );

    this.canvas.addEventListener(
      "touchmove",
      (e: TouchEvent) => {
        e.preventDefault();
        if (!this.mouseDown) return;
        const { offsetX, offsetY } = getTouchOffset(e);
        this.handleDraw(offsetX, offsetY);
      },
      { signal, passive: false },
    );

    this.canvas.addEventListener(
      "touchend",
      (e: TouchEvent) => {
        e.preventDefault();
        this.mouseDown = false;
      },
      { signal, passive: false },
    );
  }
}
