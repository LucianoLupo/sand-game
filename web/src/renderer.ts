import { Species, COLORS } from "./types";

const VERTEX_SHADER = `#version 300 es
in vec2 a_position;
out vec2 v_texCoord;

void main() {
  gl_Position = vec4(a_position, 0.0, 1.0);
  // Map clip space [-1,1] to tex coords [0,1], flip Y so top-left is (0,0)
  v_texCoord = vec2(a_position.x * 0.5 + 0.5, 1.0 - (a_position.y * 0.5 + 0.5));
}
`;

const FRAGMENT_SHADER = `#version 300 es
precision mediump float;

in vec2 v_texCoord;
out vec4 fragColor;

uniform sampler2D u_cells;

// Base colors per species (normalized 0-1)
uniform vec3 u_colorEmpty;
uniform vec3 u_colorSand;
uniform vec3 u_colorWater;
uniform vec3 u_colorOil;
uniform vec3 u_colorWall;
uniform vec3 u_colorFire;
uniform vec3 u_colorPlant;
uniform vec3 u_colorSteam;
uniform vec3 u_colorLava;
uniform vec3 u_colorStone;

void main() {
  vec4 cell = texture(u_cells, v_texCoord);

  // Decode species from red channel (stored as 0-255, normalized to 0-1)
  int species = int(cell.r * 255.0 + 0.5);
  float ra = cell.g; // ra value (0-1) for color variation

  vec3 color;

  if (species == ${Species.Empty}) {
    color = u_colorEmpty;
  } else if (species == ${Species.Sand}) {
    color = u_colorSand;
    // Vary lightness by ra for golden grain effect
    color += vec3(ra * 0.05 - 0.025);
  } else if (species == ${Species.Water}) {
    color = u_colorWater;
    // Vary blue channel slightly for shimmer
    color.b += ra * 0.06 - 0.03;
    color.g += ra * 0.02 - 0.01;
  } else if (species == ${Species.Oil}) {
    color = u_colorOil;
    // Subtle dark variation
    color += vec3(ra * 0.03 - 0.015);
  } else if (species == ${Species.Wall}) {
    color = u_colorWall;
  } else if (species == ${Species.Fire}) {
    color = u_colorFire;
    // Flicker: ra drives orange-to-yellow variation
    color.r += ra * 0.05;
    color.g += ra * 0.15 - 0.05;
  } else if (species == ${Species.Plant}) {
    color = u_colorPlant;
    // Vary green channel for organic look
    color.g += ra * 0.06 - 0.03;
    color.r += ra * 0.02 - 0.01;
  } else if (species == ${Species.Steam}) {
    color = u_colorSteam;
    // Wispy variation
    color += vec3(ra * 0.04 - 0.02);
  } else if (species == ${Species.Lava}) {
    color = u_colorLava;
    // Glow variation: shift between deep red and orange
    color.r += ra * 0.06;
    color.g += ra * 0.1 - 0.02;
  } else if (species == ${Species.Stone}) {
    color = u_colorStone;
    // Subtle grain
    color += vec3(ra * 0.04 - 0.02);
  } else {
    color = u_colorEmpty;
  }

  fragColor = vec4(clamp(color, 0.0, 1.0), 1.0);
}
`;

// Fullscreen quad: two triangles covering clip space
const QUAD_VERTICES = new Float32Array([
  -1, -1,
   1, -1,
  -1,  1,
  -1,  1,
   1, -1,
   1,  1,
]);

function compileShader(gl: WebGL2RenderingContext, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type);
  if (!shader) throw new Error("Failed to create shader");
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const info = gl.getShaderInfoLog(shader);
    gl.deleteShader(shader);
    throw new Error(`Shader compile error: ${info}`);
  }
  return shader;
}

function createProgram(gl: WebGL2RenderingContext, vs: WebGLShader, fs: WebGLShader): WebGLProgram {
  const program = gl.createProgram();
  if (!program) throw new Error("Failed to create program");
  gl.attachShader(program, vs);
  gl.attachShader(program, fs);
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    const info = gl.getProgramInfoLog(program);
    gl.deleteProgram(program);
    throw new Error(`Program link error: ${info}`);
  }
  return program;
}

function normalizeColor(rgb: [number, number, number]): [number, number, number] {
  return [rgb[0] / 255, rgb[1] / 255, rgb[2] / 255];
}

export class Renderer {
  private gl: WebGL2RenderingContext;
  private program: WebGLProgram;
  private vao: WebGLVertexArrayObject;
  private vbo: WebGLBuffer;
  private texture: WebGLTexture;
  private simWidth: number;
  private simHeight: number;
  private destroyed = false;

  constructor(canvas: HTMLCanvasElement, simWidth: number, simHeight: number) {
    this.simWidth = simWidth;
    this.simHeight = simHeight;

    const gl = canvas.getContext("webgl2", { antialias: false, alpha: false });
    if (!gl) throw new Error("WebGL2 not supported");
    this.gl = gl;

    canvas.addEventListener("webglcontextlost", (e) => {
      e.preventDefault();
      console.warn("WebGL context lost");
    });

    // Compile shaders and link program
    const vs = compileShader(gl, gl.VERTEX_SHADER, VERTEX_SHADER);
    const fs = compileShader(gl, gl.FRAGMENT_SHADER, FRAGMENT_SHADER);
    this.program = createProgram(gl, vs, fs);
    gl.deleteShader(vs);
    gl.deleteShader(fs);

    // Set up fullscreen quad VAO
    this.vao = gl.createVertexArray()!;
    this.vbo = gl.createBuffer()!;
    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vbo);
    gl.bufferData(gl.ARRAY_BUFFER, QUAD_VERTICES, gl.STATIC_DRAW);

    const posLoc = gl.getAttribLocation(this.program, "a_position");
    gl.enableVertexAttribArray(posLoc);
    gl.vertexAttribPointer(posLoc, 2, gl.FLOAT, false, 0, 0);
    gl.bindVertexArray(null);

    // Create texture for cell data
    this.texture = gl.createTexture()!;
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    // Allocate texture storage
    gl.texImage2D(
      gl.TEXTURE_2D, 0, gl.RGBA8, simWidth, simHeight, 0,
      gl.RGBA, gl.UNSIGNED_BYTE, null,
    );

    // Set up uniforms (they don't change per frame)
    gl.useProgram(this.program);
    gl.uniform1i(gl.getUniformLocation(this.program, "u_cells"), 0);

    const setColor = (name: string, species: Species) => {
      const [r, g, b] = normalizeColor(COLORS[species]);
      gl.uniform3f(gl.getUniformLocation(this.program, name), r, g, b);
    };
    setColor("u_colorEmpty", Species.Empty);
    setColor("u_colorSand", Species.Sand);
    setColor("u_colorWater", Species.Water);
    setColor("u_colorOil", Species.Oil);
    setColor("u_colorWall", Species.Wall);
    setColor("u_colorFire", Species.Fire);
    setColor("u_colorPlant", Species.Plant);
    setColor("u_colorSteam", Species.Steam);
    setColor("u_colorLava", Species.Lava);
    setColor("u_colorStone", Species.Stone);

    this.resize();
  }

  render(cellsPtr: number, memory: WebAssembly.Memory): void {
    if (this.destroyed) return;

    const gl = this.gl;
    const byteLen = this.simWidth * this.simHeight * 4;
    const cells = new Uint8Array(memory.buffer, cellsPtr, byteLen);

    // Upload cell data as texture
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    gl.texSubImage2D(
      gl.TEXTURE_2D, 0, 0, 0, this.simWidth, this.simHeight,
      gl.RGBA, gl.UNSIGNED_BYTE, cells,
    );

    // Draw fullscreen quad
    gl.bindVertexArray(this.vao);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
  }

  resize(): void {
    if (this.destroyed) return;

    const gl = this.gl;
    const canvas = gl.canvas as HTMLCanvasElement;
    const dpr = window.devicePixelRatio || 1;
    const displayWidth = Math.floor(canvas.clientWidth * dpr);
    const displayHeight = Math.floor(canvas.clientHeight * dpr);

    if (canvas.width !== displayWidth || canvas.height !== displayHeight) {
      canvas.width = displayWidth;
      canvas.height = displayHeight;
    }

    gl.viewport(0, 0, canvas.width, canvas.height);
  }

  destroy(): void {
    if (this.destroyed) return;
    this.destroyed = true;

    const gl = this.gl;
    gl.deleteTexture(this.texture);
    gl.deleteBuffer(this.vbo);
    gl.deleteVertexArray(this.vao);
    gl.deleteProgram(this.program);
  }
}
