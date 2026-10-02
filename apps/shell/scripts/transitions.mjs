// Compiles every wallpaper transition in a real browser and holds each to the
// promise `transitions.ts` makes: an opaque colour at every pixel, the old
// picture exactly at progress 0 and the new one exactly at progress 1.
//
// A shader that breaks either end does not fail loudly — it flashes for one
// frame at the start or end of every wallpaper change. Three of the first six
// did, unnoticed, until this compared them pixel for pixel.
//
//     pnpm --filter @bw/shell check:transitions

import { existsSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { chromium } from "@playwright/test";
import { transformWithEsbuild } from "vite";

const PRESET_CHROMIUM = process.env.BW_CHROMIUM ?? "/opt/pw-browsers/chromium";
// A runner has no GPU. SwiftShader renders WebGL on the CPU, and recent
// Chromium only falls back to it when told it may.
const args = [
  "--no-sandbox",
  "--use-angle=swiftshader",
  "--enable-unsafe-swiftshader",
];
const launchOptions = existsSync(PRESET_CHROMIUM)
  ? { executablePath: PRESET_CHROMIUM, args }
  : { args };

const source = await readFile(
  new URL("../src/gl/transitions.ts", import.meta.url),
  "utf8",
);
const { code } = await transformWithEsbuild(source, "transitions.ts", {
  format: "esm",
});

const browser = await chromium.launch(launchOptions);
const page = await browser.newPage();
const results = await page.evaluate(async (module) => {
  const url = URL.createObjectURL(
    new Blob([module], { type: "text/javascript" }),
  );
  const { VERTEX_SHADER, TRANSITION_NAMES, fragmentShaderFor } = await import(
    url
  );

  const W = 160;
  const H = 90;
  const canvas = document.createElement("canvas");
  canvas.width = W;
  canvas.height = H;
  const gl = canvas.getContext("webgl", { preserveDrawingBuffer: true });
  if (!gl) return { error: "no WebGL" };

  // Two pictures that differ everywhere, with detail for a shader to move.
  const picture = (hue, word) => {
    const c = document.createElement("canvas");
    c.width = 400;
    c.height = 300;
    const g = c.getContext("2d");
    const fill = g.createLinearGradient(0, 0, 400, 300);
    fill.addColorStop(0, `hsl(${hue},80%,30%)`);
    fill.addColorStop(1, `hsl(${hue + 40},80%,70%)`);
    g.fillStyle = fill;
    g.fillRect(0, 0, 400, 300);
    g.fillStyle = "#fff";
    g.font = "bold 90px sans-serif";
    g.fillText(word, 90, 180);
    const texture = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, texture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, c);
    return texture;
  };
  const from = picture(10, "OLD");
  const to = picture(200, "NEW");

  gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(
    gl.ARRAY_BUFFER,
    new Float32Array([-1, -1, 3, -1, -1, 3]),
    gl.STATIC_DRAW,
  );

  const build = (fragment) => {
    const stage = (type, text) => {
      const shader = gl.createShader(type);
      gl.shaderSource(shader, text);
      gl.compileShader(shader);
      if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS))
        throw new Error(gl.getShaderInfoLog(shader));
      return shader;
    };
    const program = gl.createProgram();
    gl.attachShader(program, stage(gl.VERTEX_SHADER, VERTEX_SHADER));
    gl.attachShader(program, stage(gl.FRAGMENT_SHADER, fragment));
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS))
      throw new Error(gl.getProgramInfoLog(program));
    return program;
  };

  const draw = (program, progress, seed) => {
    gl.viewport(0, 0, W, H);
    gl.useProgram(program);
    const position = gl.getAttribLocation(program, "aPosition");
    gl.enableVertexAttribArray(position);
    gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);
    const at = (name) => gl.getUniformLocation(program, name);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, from);
    gl.uniform1i(at("uFrom"), 0);
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, to);
    gl.uniform1i(at("uTo"), 1);
    gl.uniform1f(at("uProgress"), progress);
    gl.uniform2f(at("uResolution"), W, H);
    gl.uniform1f(at("uSeed"), seed);
    gl.uniform2f(at("uFromSize"), 400, 300);
    gl.uniform2f(at("uToSize"), 400, 300);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    const pixels = new Uint8Array(W * H * 4);
    gl.readPixels(0, 0, W, H, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
    return pixels;
  };

  // Pixels more than a rounding error away from the reference, or not opaque.
  const wrong = (pixels, reference) => {
    let count = 0;
    for (let i = 0; i < pixels.length; i += 4) {
      const off =
        Math.abs(pixels[i] - reference[i]) > 2 ||
        Math.abs(pixels[i + 1] - reference[i + 1]) > 2 ||
        Math.abs(pixels[i + 2] - reference[i + 2]) > 2;
      if (off || pixels[i + 3] !== 255) count++;
    }
    return count;
  };
  const translucent = (pixels) => {
    let count = 0;
    for (let i = 3; i < pixels.length; i += 4) if (pixels[i] !== 255) count++;
    return count;
  };

  // The plain crossfade is the reference: at its ends it is the two
  // pictures and nothing else.
  const fade = build(fragmentShaderFor("fade"));
  const problems = [];
  for (const name of TRANSITION_NAMES) {
    let program;
    try {
      program = build(fragmentShaderFor(name));
    } catch (error) {
      problems.push(`${name}: does not compile: ${error.message}`);
      continue;
    }
    // A few seeds, since several shaders place things by them.
    for (const seed of [3.1, 57.7, 91.2]) {
      const start = wrong(draw(program, 0, seed), draw(fade, 0, seed));
      const end = wrong(draw(program, 1, seed), draw(fade, 1, seed));
      const middle = [0.25, 0.5, 0.75]
        .map((t) => translucent(draw(program, t, seed)))
        .reduce((a, b) => a + b, 0);
      if (start) problems.push(`${name}: ${start} pixels wrong at progress 0`);
      if (end) problems.push(`${name}: ${end} pixels wrong at progress 1`);
      if (middle) problems.push(`${name}: ${middle} translucent pixels`);
      if (start || end || middle) break;
    }
  }
  return { names: TRANSITION_NAMES, problems };
}, code);
await browser.close();

if (results.error) {
  console.error(results.error);
  process.exitCode = 1;
} else if (results.problems.length) {
  console.error("Transitions that break their promise:");
  for (const problem of results.problems) console.error(`  ${problem}`);
  process.exitCode = 1;
} else {
  console.log(`${results.names.length} transitions hold at both ends.`);
}
