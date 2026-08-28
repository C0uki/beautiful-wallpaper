// Synthetic wallpapers for the mock backend.
//
// Real photographs cannot be committed to the repository, and the shell needs
// *something* to render, crossfade and derive colours from. These are generated
// SVG gradients with a little deterministic noise so they read as distinct
// images and the transition shaders have something to chew on.

function hashOf(seed: string): number {
  let hash = 2166136261;
  for (let index = 0; index < seed.length; index += 1) {
    hash ^= seed.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return Math.abs(hash);
}

/** A 16:9 gradient with drifting blobs, as a data URL. */
export function gradientWallpaper(
  from: string,
  to: string,
  seed: string,
): string {
  const hash = hashOf(seed);
  const angle = hash % 360;
  const blobs = Array.from({ length: 4 }, (_, index) => {
    const local = hashOf(`${seed}-${index}`);
    const cx = 200 + (local % 1520);
    const cy = 120 + ((local >> 8) % 840);
    const r = 200 + ((local >> 16) % 420);
    const opacity = 0.1 + ((local >> 4) % 18) / 100;
    return `<circle cx="${cx}" cy="${cy}" r="${r}" fill="url(#blob)" opacity="${opacity.toFixed(2)}" />`;
  }).join("");

  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1920 1080" width="1920" height="1080">
  <defs>
    <linearGradient id="bg" gradientTransform="rotate(${angle} 0.5 0.5)">
      <stop offset="0%" stop-color="${from}" />
      <stop offset="100%" stop-color="${to}" />
    </linearGradient>
    <radialGradient id="blob">
      <stop offset="0%" stop-color="${to}" stop-opacity="0.9" />
      <stop offset="100%" stop-color="${to}" stop-opacity="0" />
    </radialGradient>
  </defs>
  <rect width="1920" height="1080" fill="url(#bg)" />
  ${blobs}
</svg>`;

  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

/** A stand-in for "what is on the screen right now", as a data URL.
 *
 * The region picker draws a frozen copy of the screen, so the mock has to
 * supply one. This is a plainly synthetic desktop — a gradient, a window and
 * some lines of text — which is enough for the overlay's shading, its
 * dimension readout and its result panel to be looked at off Windows. */
export function mockScreen(): string {
  const lines = [
    "Windows has a text recogniser built in.",
    "It only exists for languages whose pack is installed,",
    "so the shell asks before it offers to read anything.",
  ];
  const text = lines
    .map(
      (line, index) =>
        `<text x="228" y="${304 + index * 34}" font-family="Segoe UI, sans-serif" font-size="21" fill="#1c1a22">${line}</text>`,
    )
    .join("");

  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080">
    <defs><linearGradient id="d" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#1b2436"/><stop offset="1" stop-color="#4a3350"/>
    </linearGradient></defs>
    <rect width="1920" height="1080" fill="url(#d)"/>
    <rect x="200" y="180" width="1120" height="420" rx="14" fill="#f4f1f7"/>
    <rect x="200" y="180" width="1120" height="44" rx="14" fill="#e2dee8"/>
    <circle cx="228" cy="202" r="7" fill="#d98a8a"/>
    <circle cx="252" cy="202" r="7" fill="#d9c58a"/>
    <circle cx="276" cy="202" r="7" fill="#8ad99a"/>
    <text x="228" y="264" font-family="Segoe UI, sans-serif" font-size="26" font-weight="600" fill="#1c1a22">Reading text off the screen</text>
    ${text}
    <rect x="1420" y="700" width="380" height="240" rx="14" fill="#2a2333" opacity="0.85"/>
  </svg>`;

  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}
