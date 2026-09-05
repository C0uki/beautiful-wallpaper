// The wallpaper renderer.
//
// Two textures and a progress value, drawn by whichever transition shader the
// config names. On Windows this canvas lives in the window parented to WorkerW,
// which puts it above the desktop wallpaper and below the icons.
//
// If WebGL is unavailable — a stripped-down VM, a GPU driver refusing to create a
// context — it falls back to two stacked images with a CSS crossfade, so the
// shell still shows a wallpaper.

import { useCallback, useEffect, useRef, useState } from "react";
import { VERTEX_SHADER, fragmentShaderFor } from "./transitions";

export interface WallpaperCanvasProps {
  /** The image to show. Changing it starts a transition from the previous one. */
  src: string;
  transition: string;
  durationMs: number;
  /** Zoom and pan, for the parallax effect. */
  zoom?: number;
  panX?: number;
  panY?: number;
  className?: string;
}

interface Program {
  gl: WebGLRenderingContext;
  program: WebGLProgram;
  uniforms: Record<string, WebGLUniformLocation | null>;
}

function compile(
  gl: WebGLRenderingContext,
  type: number,
  source: string,
): WebGLShader | null {
  const shader = gl.createShader(type);
  if (!shader) return null;
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    console.error(
      "wallpaper shader failed to compile:",
      gl.getShaderInfoLog(shader),
    );
    gl.deleteShader(shader);
    return null;
  }
  return shader;
}

function buildProgram(
  gl: WebGLRenderingContext,
  fragmentSource: string,
): Program | null {
  const vertex = compile(gl, gl.VERTEX_SHADER, VERTEX_SHADER);
  const fragment = compile(gl, gl.FRAGMENT_SHADER, fragmentSource);
  if (!vertex || !fragment) return null;

  const program = gl.createProgram();
  if (!program) return null;
  gl.attachShader(program, vertex);
  gl.attachShader(program, fragment);
  gl.linkProgram(program);
  gl.deleteShader(vertex);
  gl.deleteShader(fragment);

  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    console.error(
      "wallpaper program failed to link:",
      gl.getProgramInfoLog(program),
    );
    return null;
  }

  const uniforms = Object.fromEntries(
    [
      "uFrom",
      "uTo",
      "uProgress",
      "uResolution",
      "uSeed",
      "uFromSize",
      "uToSize",
    ].map((name) => [name, gl.getUniformLocation(program, name)]),
  );

  return { gl, program, uniforms };
}

function uploadTexture(
  gl: WebGLRenderingContext,
  image: HTMLImageElement,
  existing?: WebGLTexture | null,
): WebGLTexture | null {
  const texture = existing ?? gl.createTexture();
  if (!texture) return null;
  gl.bindTexture(gl.TEXTURE_2D, texture);
  // Wallpapers are rarely powers of two, so clamp and use linear filtering only.
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
  return texture;
}

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.crossOrigin = "anonymous";
    image.decoding = "async";
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error(`could not load wallpaper: ${src}`));
    image.src = src;
  });
}

export function WallpaperCanvas({
  src,
  transition,
  durationMs,
  zoom = 1,
  panX = 0,
  panY = 0,
  className,
}: WallpaperCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [webglFailed, setWebglFailed] = useState(false);

  // Kept in refs: the render loop reads them every frame and must not restart.
  const stateRef = useRef({
    from: null as WebGLTexture | null,
    to: null as WebGLTexture | null,
    fromSize: [1, 1] as [number, number],
    toSize: [1, 1] as [number, number],
    progress: 1,
    startedAt: 0,
    duration: durationMs,
    seed: Math.random() * 100,
    animating: false,
  });
  const programRef = useRef<Program | null>(null);
  const currentSrcRef = useRef<string>("");

  // A wallpaper spends almost all of its life sitting still, and a frame drawn
  // then is identical to the one before it. Driving requestAnimationFrame
  // through that pins a full-screen shader to the refresh rate — on a 144Hz
  // panel, 144 redraws a second of a picture that has not changed — and keeps
  // the GPU from ever idling. So nothing is drawn unless something asks for it:
  // a running transition asks for the next frame, and the three things that can
  // change a still frame — a new texture, a new shader, a resize — ask once.
  const frameRef = useRef(0);
  const drawRef = useRef<(() => void) | null>(null);
  const requestRender = useCallback(() => {
    if (frameRef.current) return;
    frameRef.current = requestAnimationFrame(() => {
      frameRef.current = 0;
      drawRef.current?.();
    });
  }, []);

  // Set up the context once. Rebuilding it on every wallpaper change would drop
  // the outgoing texture, which is exactly what the transition needs.
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const gl = canvas.getContext("webgl", {
      alpha: false,
      antialias: false,
      premultipliedAlpha: false,
      preserveDrawingBuffer: false,
      powerPreference: "low-power",
    });
    if (!gl) {
      setWebglFailed(true);
      return;
    }

    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([-1, -1, 3, -1, -1, 3]),
      gl.STATIC_DRAW,
    );

    const draw = () => {
      const program = programRef.current;
      const state = stateRef.current;
      if (!program || !state.to) return;

      if (state.animating) {
        const elapsed = performance.now() - state.startedAt;
        state.progress = Math.min(1, elapsed / Math.max(1, state.duration));
        if (state.progress >= 1) state.animating = false;
      }

      const { clientWidth, clientHeight } = canvas;
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      const width = Math.max(1, Math.round(clientWidth * dpr));
      const height = Math.max(1, Math.round(clientHeight * dpr));
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
      gl.viewport(0, 0, width, height);

      gl.useProgram(program.program);
      const position = gl.getAttribLocation(program.program, "aPosition");
      gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
      gl.enableVertexAttribArray(position);
      gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);

      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, state.from ?? state.to);
      gl.uniform1i(program.uniforms["uFrom"] ?? null, 0);
      gl.activeTexture(gl.TEXTURE1);
      gl.bindTexture(gl.TEXTURE_2D, state.to);
      gl.uniform1i(program.uniforms["uTo"] ?? null, 1);

      gl.uniform1f(program.uniforms["uProgress"] ?? null, state.progress);
      gl.uniform2f(program.uniforms["uResolution"] ?? null, width, height);
      gl.uniform1f(program.uniforms["uSeed"] ?? null, state.seed);
      gl.uniform2f(program.uniforms["uFromSize"] ?? null, ...state.fromSize);
      gl.uniform2f(program.uniforms["uToSize"] ?? null, ...state.toSize);

      gl.drawArrays(gl.TRIANGLES, 0, 3);

      // Only a transition still in flight needs the frame after this one.
      if (state.animating) requestRender();
    };

    drawRef.current = draw;

    // A resize changes the drawing buffer, and the canvas is `inset: 0`, so this
    // also covers a scale change: at a fixed physical size, a different device
    // pixel ratio is a different CSS size.
    const observer = new ResizeObserver(requestRender);
    observer.observe(canvas);

    requestRender();

    return () => {
      observer.disconnect();
      if (frameRef.current) cancelAnimationFrame(frameRef.current);
      frameRef.current = 0;
      drawRef.current = null;
      gl.deleteBuffer(buffer);
    };
  }, [requestRender]);

  // Recompile when the configured transition changes.
  useEffect(() => {
    const canvas = canvasRef.current;
    const gl = canvas?.getContext("webgl");
    if (!gl) return;
    const built = buildProgram(gl, fragmentShaderFor(transition));
    if (built) programRef.current = built;
    requestRender();
  }, [transition, requestRender]);

  // Swap in a new wallpaper, keeping the old texture to transition away from.
  useEffect(() => {
    if (!src || src === currentSrcRef.current) return;
    currentSrcRef.current = src;

    let cancelled = false;
    void (async () => {
      const canvas = canvasRef.current;
      const gl = canvas?.getContext("webgl");
      if (!gl) return;

      let image: HTMLImageElement;
      try {
        image = await loadImage(src);
      } catch (error) {
        console.error(error);
        return;
      }
      if (cancelled) return;

      const state = stateRef.current;
      // The outgoing image becomes `from`; the incoming one becomes `to`.
      if (state.from) gl.deleteTexture(state.from);
      state.from = state.to;
      state.fromSize = state.toSize;
      state.to = uploadTexture(gl, image);
      state.toSize = [image.naturalWidth || 1, image.naturalHeight || 1];
      state.seed = Math.random() * 100;
      state.duration = durationMs;

      if (state.from) {
        state.progress = 0;
        state.startedAt = performance.now();
        state.animating = true;
      } else {
        // First wallpaper of the session: show it outright rather than fading in
        // from nothing.
        state.progress = 1;
        state.animating = false;
      }

      requestRender();
    })();

    return () => {
      cancelled = true;
    };
  }, [src, durationMs, requestRender]);

  const transform = `scale(${zoom}) translate(${panX}%, ${panY}%)`;

  if (webglFailed) {
    return (
      <div
        className={className}
        style={{
          position: "absolute",
          inset: 0,
          backgroundImage: `url("${src}")`,
          backgroundSize: "cover",
          backgroundPosition: "center",
          transform,
          transition: `background-image ${durationMs}ms var(--ease-effects-default)`,
        }}
      />
    );
  }

  return (
    <canvas
      ref={canvasRef}
      className={className}
      style={{
        position: "absolute",
        inset: 0,
        width: "100%",
        height: "100%",
        display: "block",
        transform,
        transition:
          "transform var(--duration-slower) var(--ease-spatial-default)",
      }}
    />
  );
}
