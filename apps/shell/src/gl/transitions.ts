// Wallpaper transition shaders.
//
// end4-pC ships fourteen of these as precompiled Qt `.qsb` bundles, which have no
// GLSL source in the repository and no meaning outside Qt's renderer. These are
// re-authored as WebGL fragment shaders against the same names, so a config that
// says `wallpaperAnimation: "circle"` keeps meaning what it meant. Written from
// the names and what they do on screen, not from the bundles, so they are
// approximations of the originals rather than reproductions.
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
  float edge = 0.04 + 0.10 * (1.0 - abs(uProgress * 2.0 - 1.0));
  // From one edge-width short of the centre, so nothing of the new image
  // shows at progress 0, to past the far corner at progress 1.
  float radius = mix(-0.04, length(0.5 * aspect) * 1.05, uProgress);
  float mask = smoothstep(radius + edge, radius - edge, dist);
  gl_FragColor = mix(fromColor(vUv), toColor(vUv), mask);
}
`;

/** Per-pixel noise threshold: the new image eats the old one in speckles. */
const DISSOLVE = `
void main() {
  float noise = hash(floor(vUv * uResolution / 3.0));
  // Widen the band so the change is gradual rather than a hard cut, and
  // stretch the progress past it at both ends so no speck has started at 0
  // or is left over at 1.
  float progress = uProgress * 1.3 - 0.15;
  float mask = smoothstep(noise - 0.15, noise + 0.15, progress);
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
  // Starting just short of the centre, so nothing has moved at progress 0.
  float front = uProgress * 1.4 - 0.05;
  // The displacement lives only in a narrow band around the wavefront, and
  // is gone at both ends.
  float band = smoothstep(0.18, 0.0, abs(dist - front));
  float amplitude = 0.09 * band * (1.0 - abs(uProgress * 2.0 - 1.0));
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

/** Bands of the picture jump sideways with their colours split, and each
 *  band switches over on its own frame. */
const GLITCH = `
void main() {
  float bell = 1.0 - abs(uProgress * 2.0 - 1.0);
  float frame = floor(uProgress * 20.0);
  float band = floor(vUv.y * 24.0 + frame * 7.0);
  float jump = (hash(vec2(band, frame)) - 0.5) * 0.12 * bell
    * step(0.55, hash(vec2(band * 3.1, frame)));
  vec2 uv = vec2(vUv.x + jump, vUv.y);
  vec2 split = vec2(0.012 * bell, 0.0);
  // Every band has crossed by progress 1: the threshold is below 1.
  float crossed = step(hash(vec2(band, frame + 1.0)), uProgress);
  vec3 from = vec3(fromColor(uv + split).r, fromColor(uv).g, fromColor(uv - split).b);
  vec3 to = vec3(toColor(uv + split).r, toColor(uv).g, toColor(uv - split).b);
  gl_FragColor = vec4(mix(from, to, crossed), 1.0);
}
`;

/** An old television: the picture collapses to a line, then a dot, and the
 *  new one opens back out of it. */
const CRT = `
void main() {
  // 0 at both ends, 1 at the dot in the middle.
  float phase = uProgress < 0.5 ? uProgress * 2.0 : (1.0 - uProgress) * 2.0;
  vec2 squeeze = vec2(
    max(1.0 - smoothstep(0.7, 1.0, phase), 0.004),
    max(1.0 - smoothstep(0.0, 0.7, phase), 0.004)
  );
  vec2 uv = (vUv - 0.5) / squeeze + 0.5;
  bool inside = all(greaterThanEqual(uv, vec2(0.0))) && all(lessThanEqual(uv, vec2(1.0)));
  vec3 picture = (uProgress < 0.5 ? fromColor(uv) : toColor(uv)).rgb + phase * 0.6;
  float scanline = 1.0 - 0.15 * phase * step(0.5, fract(vUv.y * uResolution.y / 3.0));
  gl_FragColor = inside ? vec4(picture * scanline, 1.0) : vec4(0.0, 0.0, 0.0, 1.0);
}
`;

/** The old picture breaks into cells that shrink away one after another. */
const SHATTER = `
const float CELLS = 9.0;

// The site of the cell nearest to p, among a jittered grid's.
vec2 nearestSite(vec2 p) {
  vec2 base = floor(p);
  float best = 1e9;
  vec2 site = vec2(0.0);
  for (int y = -1; y <= 1; y++) {
    for (int x = -1; x <= 1; x++) {
      vec2 cell = base + vec2(float(x), float(y));
      vec2 candidate = cell + 0.15 + 0.7 * vec2(hash(cell), hash(cell + 17.3));
      float distance2 = dot(p - candidate, p - candidate);
      if (distance2 < best) {
        best = distance2;
        site = candidate;
      }
    }
  }
  return site;
}

void main() {
  vec2 aspect = vec2(uResolution.x / uResolution.y, 1.0);
  vec2 p = vUv * aspect * CELLS;
  vec2 site = nearestSite(p);
  // Every cell starts by progress 0.5 and is gone half a run later.
  float local = clamp((uProgress - hash(floor(site) + 3.7) * 0.5) / 0.5, 0.0, 1.0);
  float scale = 1.0 - local;
  if (scale <= 0.0) {
    gl_FragColor = toColor(vUv);
    return;
  }
  // A cell shrinks towards its own site, so it stays inside itself: what
  // this pixel shows is the point it came from, if that was in the cell.
  vec2 origin = site + (p - site) / scale;
  if (distance(nearestSite(origin), site) > 1e-3) {
    gl_FragColor = toColor(vUv);
    return;
  }
  vec4 piece = fromColor(origin / CELLS / aspect);
  gl_FragColor = vec4(piece.rgb * (1.0 - 0.35 * local), 1.0);
}
`;

/** Doom's screen melt: the old picture slides down in uneven columns. */
const DOOM = `
void main() {
  const float COLUMNS = 160.0;
  float column = floor(vUv.x * COLUMNS);
  // Neighbouring columns start close together, as in the original's random
  // walk, with a little noise of their own.
  float coarse = column / 8.0;
  float delay = mix(
    hash(vec2(floor(coarse), 0.0)),
    hash(vec2(floor(coarse) + 1.0, 0.0)),
    fract(coarse)
  ) * 0.2 + hash(vec2(column, 5.0)) * 0.05;
  float local = clamp((uProgress - delay) / 0.75, 0.0, 1.0);
  // y runs upwards, so the old picture falling means reading from above.
  float y = vUv.y + local * local * 1.02;
  gl_FragColor = y < 1.0 ? fromColor(vec2(vUv.x, y)) : toColor(vUv);
}
`;

/** A swirl that winds up and unwinds again, with the swap hidden in it and
 *  sparks carried round. */
const MAGIC = `
void main() {
  vec2 aspect = vec2(uResolution.x / uResolution.y, 1.0);
  vec2 centred = (vUv - 0.5) * aspect;
  float bell = 1.0 - abs(uProgress * 2.0 - 1.0);
  float angle = bell * 6.0 * (1.0 - smoothstep(0.0, 0.8, length(centred)));
  float s = sin(angle);
  float c = cos(angle);
  vec2 twisted = vec2(c * centred.x - s * centred.y, s * centred.x + c * centred.y) / aspect + 0.5;
  vec4 color = mix(fromColor(twisted), toColor(twisted), smoothstep(0.35, 0.65, uProgress));
  float spark = step(0.985, hash(floor(twisted * uResolution / 4.0))) * bell;
  gl_FragColor = vec4(color.rgb + spark * 0.8, 1.0);
}
`;

/** The old picture peels away from the bottom-right corner like a page,
 *  its back folded over what is still flat. */
const PEEL = `
void main() {
  vec2 aspect = vec2(uResolution.x / uResolution.y, 1.0);
  vec2 p = vUv * aspect;
  vec2 corner = vec2(aspect.x, 0.0);
  vec2 towards = normalize(vec2(-aspect.x, 1.0));
  // The fold passes the far corner just before progress 1.
  float fold = uProgress * (dot(vec2(0.0, 1.0) - corner, towards) + 0.01);
  float along = dot(p - corner, towards);
  if (along < fold) {
    // Lifted: the new picture, in the flap's shadow near the fold.
    float shade = 1.0 - 0.4 * (1.0 - uProgress) * (1.0 - smoothstep(0.0, 0.08, fold - along));
    gl_FragColor = vec4(toColor(vUv).rgb * shade, 1.0);
    return;
  }
  vec2 mirrored = (p - 2.0 * (along - fold) * towards) / aspect;
  bool onFlap = along < 2.0 * fold
    && all(greaterThanEqual(mirrored, vec2(0.0)))
    && all(lessThanEqual(mirrored, vec2(1.0)));
  if (onFlap) {
    vec3 back = fromColor(mirrored).rgb;
    back = mix(back, vec3(dot(back, vec3(0.299, 0.587, 0.114))), 0.6) * 0.85 + 0.12;
    gl_FragColor = vec4(back, 1.0);
    return;
  }
  gl_FragColor = fromColor(vUv);
}
`;

/** The old picture twists down into a shrinking pit in the middle. */
const CIRCLE_PIT = `
void main() {
  vec2 aspect = vec2(uResolution.x / uResolution.y, 1.0);
  vec2 centred = (vUv - 0.5) * aspect;
  float reach = length(0.5 * aspect) * 1.05;
  float radius = (1.0 - uProgress) * reach;
  float dist = length(centred);
  if (dist >= radius) {
    gl_FragColor = toColor(vUv);
    return;
  }
  float twist = uProgress * 4.0 * (1.0 - dist / radius);
  float s = sin(twist);
  float c = cos(twist);
  vec2 q = centred / max(radius / reach, 1e-3);
  q = vec2(c * q.x - s * q.y, s * q.x + c * q.y);
  float rim = smoothstep(radius - 0.03, radius, dist);
  gl_FragColor = mix(fromColor(q / aspect + 0.5), toColor(vUv), rim);
}
`;

/** A selection circle drawn out from somewhere on the screen, outlined while
 *  it grows, with the new picture inside it. */
const CIRCLE_SELECT = `
void main() {
  vec2 aspect = vec2(uResolution.x / uResolution.y, 1.0);
  // Somewhere away from the edges, a different place each time.
  vec2 origin = vec2(hash(vec2(1.0, 2.0)), hash(vec2(3.0, 4.0))) * 0.6 + 0.2;
  float far = length(max(origin, 1.0 - origin) * aspect);
  float radius = uProgress * far * 1.02;
  float dist = length((vUv - origin) * aspect);
  vec4 color = dist < radius ? toColor(vUv) : fromColor(vUv);
  float outline = smoothstep(radius - 0.006, radius, dist)
    - smoothstep(radius, radius + 0.006, dist);
  float bell = 1.0 - abs(uProgress * 2.0 - 1.0);
  gl_FragColor = vec4(mix(color.rgb, vec3(1.0), outline * bell * 0.8), 1.0);
}
`;

export const TRANSITIONS: Record<string, string> = {
  fade: FADE,
  circle: CIRCLE,
  dissolve: DISSOLVE,
  pixelate: PIXELATE,
  ripple: RIPPLE,
  stripes: STRIPES,
  glitch: GLITCH,
  crt: CRT,
  shatter: SHATTER,
  // Capitalised as the original spells them, so a config written for it
  // still names a transition here.
  Doom: DOOM,
  magic: MAGIC,
  Peel: PEEL,
  circlePit: CIRCLE_PIT,
  circleSelect: CIRCLE_SELECT,
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
