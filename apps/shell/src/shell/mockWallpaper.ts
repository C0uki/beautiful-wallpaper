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
