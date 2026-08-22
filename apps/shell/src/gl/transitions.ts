// Wallpaper transition shaders.
//
// end4-pC ships fourteen of these as precompiled Qt `.qsb` bundles, which have no
// GLSL source in the repository and no meaning outside Qt's renderer. These are
// re-authored as WebGL fragment shaders against the same names, so a config that
// says `wallpaperAnimation: "circle"` keeps meaning what it meant.
//
// Every shader receives both images and a 0..1 progress, and must return an
// opaque colour at every pixel for every progress — a transition that flashes the
// page background at t=0 or t=1 reads as a bug.

export const VERTEX_SHADER = `
attribute vec2 aPosition;
varying vec2 vUv;
void main() {
  vUv = aPosition * 0.5 + 0.5;
  gl_Position = vec4(aPosition, 0.0, 1.0);
}
`;

const PRELUDE = `
precision highp float;
varying vec2 vUv;
uniform sampler2D uFrom;
uniform sampler2D uTo;
uniform float uProgress;
uniform vec2 uResolution;
uniform float uSeed;

// Both images are drawn with "cover" framing, so a 4:3 wallpaper on a 16:9
// screen crops rather than stretches.
vec2 coverUv(vec2 uv, vec2 imageSize) {
  float screenAspect = uResolution.x / uResolution.y;
  float imageAspect = imageSize.x / max(imageSize.y, 1.0);
  vec2 scale = screenAspect > imageAspect
    ? vec2(1.0, imageAspect / screenAspect)
    : vec2(screenAspect / imageAspect, 1.0);
  return (uv - 0.5) / scale + 0.5;
}

uniform vec2 uFromSize;
uniform vec2 uToSize;

vec4 fromColor(vec2 uv) { return texture2D(uFrom, coverUv(uv, uFromSize)); }
vec4 toColor(vec2 uv)   { return texture2D(uTo, coverUv(uv, uToSize)); }

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7)) + uSeed) * 43758.5453123);
}
`;

/** A plain crossfade — the fallback when a name is not recognised. */
const FADE = `
void main() {
  gl_FragColor = mix(fromColor(vUv), toColor(vUv), uProgress);
}
`;

/** A circle growing from the centre, with a soft edge. */
const CIRCLE = `
void main() {
  vec2 aspect = vec2(uResolution.x / uResolution.y, 1.0);
  float dist = length((vUv - 0.5) * aspect);
  // Reach the far corner exactly at progress 1.
  float radius = uProgress * length(0.5 * aspect) * 1.05;
  float edge = 0.04 + 0.10 * (1.0 - abs(uProgress * 2.0 - 1.0));
  float mask = smoothstep(radius + edge, radius - edge, dist);
  gl_FragColor = mix(fromColor(vUv), toColor(vUv), mask);
}
`;

/** Per-pixel noise threshold: the new image eats the old one in speckles. */
const DISSOLVE = `
void main() {
  float noise = hash(floor(vUv * uResolution / 3.0));
  // Widen the band so the change is gradual rather than a hard cut.
  float mask = smoothstep(noise - 0.15, noise + 0.15, uProgress);
  gl_FragColor = mix(fromColor(vUv), toColor(vUv), mask);
}
`;

/** Blocks grow, then shrink, hiding the swap at peak coarseness. */
const PIXELATE = `
void main() {
  float bell = 1.0 - abs(uProgress * 2.0 - 1.0);
  float blocks = mix(uResolution.y, 14.0, bell * bell);
  vec2 grid = max(vec2(blocks * uResolution.x / uResolution.y, blocks), vec2(1.0));
  vec2 snapped = (floor(vUv * grid) + 0.5) / grid;
  gl_FragColor = mix(fromColor(snapped), toColor(snapped), step(0.5, uProgress));
}
`;

/** A wave travelling out from the centre, displacing as it passes. */
const RIPPLE = `
void main() {
  vec2 aspect = vec2(uResolution.x / uResolution.y, 1.0);
  vec2 centred = (vUv - 0.5) * aspect;
  float dist = length(centred);
  float front = uProgress * 1.3;
  // The displacement lives only in a narrow band around the wavefront.
  float band = smoothstep(0.18, 0.0, abs(dist - front));
  float amplitude = 0.045 * band * (1.0 - uProgress);
  vec2 offset = normalize(centred + 1e-6) * sin((dist - front) * 42.0) * amplitude;
  float mask = smoothstep(front + 0.05, front - 0.05, dist);
  gl_FragColor = mix(fromColor(vUv + offset), toColor(vUv + offset), mask);
}
`;

/** Vertical stripes sweeping down, each starting a little after the last. */
const STRIPES = `
void main() {
  const float COUNT = 14.0;
  float column = floor(vUv.x * COUNT);
  float delay = fract(sin(column * 12.9898 + uSeed) * 43758.5453) * 0.45;
  // Rescale so every stripe still finishes exactly at progress 1.
  float local = clamp((uProgress - delay) / (1.0 - delay), 0.0, 1.0);
  float mask = step(1.0 - vUv.y, local);
  gl_FragColor = mix(fromColor(vUv), toColor(vUv), mask);
}
`;

export const TRANSITIONS: Record<string, string> = {
  fade: FADE,
  circle: CIRCLE,
  dissolve: DISSOLVE,
  pixelate: PIXELATE,
  ripple: RIPPLE,
  stripes: STRIPES,
};

export const TRANSITION_NAMES = Object.keys(TRANSITIONS);

/** Resolves a configured name, including `"random"`, to a shader source. */
export function fragmentShaderFor(name: string): string {
  if (name === "random") {
    const pick =
      TRANSITION_NAMES[Math.floor(Math.random() * TRANSITION_NAMES.length)];
    return PRELUDE + (TRANSITIONS[pick ?? "fade"] ?? FADE);
  }
  return PRELUDE + (TRANSITIONS[name] ?? FADE);
}
