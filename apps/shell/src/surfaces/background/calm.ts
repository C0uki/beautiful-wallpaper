// Finding the calmest part of a wallpaper, for `placementStrategy: "leastBusy"`.
//
// The original runs a 399-line OpenCV script over the image. What it measures
// is how much is going on under a widget-sized box, and that is the gradient
// of the brightness summed over the box: flat sky scores low, foliage and
// text score high. A summed-area table makes every box one lookup, so trying
// every position of a small copy of the wallpaper costs nothing worth timing.

/** A box in the same pixels as the map it was found in. */
export interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** How busy each pixel is: the gradient magnitude of the brightness. */
export function busyness(
  luma: Float32Array,
  width: number,
  height: number,
): Float32Array {
  const busy = new Float32Array(width * height);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const at = (dx: number, dy: number) =>
        luma[
          Math.min(height - 1, Math.max(0, y + dy)) * width +
            Math.min(width - 1, Math.max(0, x + dx))
        ]!;
      busy[y * width + x] =
        Math.abs(at(1, 0) - at(-1, 0)) + Math.abs(at(0, 1) - at(0, -1));
    }
  }
  return busy;
}

const overlaps = (a: Box, b: Box) =>
  a.x < b.x + b.width &&
  b.x < a.x + a.width &&
  a.y < b.y + b.height &&
  b.y < a.y + a.height;

/**
 * The top-left corner of the calmest `width` × `height` box in `busy` that
 * keeps `margin` pixels from every edge and overlaps nothing in `taken`.
 * When every position overlaps something, the overlap is allowed rather than
 * the widget left without a place.
 */
export function calmestSpot(
  busy: Float32Array,
  mapWidth: number,
  mapHeight: number,
  boxWidth: number,
  boxHeight: number,
  taken: Box[],
  margin = 0,
): { x: number; y: number } {
  // A box bigger than the picture is measured as the picture.
  const width = Math.min(boxWidth, mapWidth);
  const height = Math.min(boxHeight, mapHeight);
  // Summed-area table, one wider and taller so every lookup is in range.
  const stride = mapWidth + 1;
  const sum = new Float64Array(stride * (mapHeight + 1));
  for (let y = 0; y < mapHeight; y++) {
    let row = 0;
    for (let x = 0; x < mapWidth; x++) {
      row += busy[y * mapWidth + x]!;
      sum[(y + 1) * stride + x + 1] = sum[y * stride + x + 1]! + row;
    }
  }
  const total = (x: number, y: number) =>
    sum[(y + height) * stride + x + width]! -
    sum[y * stride + x + width]! -
    sum[(y + height) * stride + x]! +
    sum[y * stride + x]!;

  const lowX = Math.min(margin, Math.max(0, mapWidth - width));
  const lowY = Math.min(margin, Math.max(0, mapHeight - height));
  const highX = Math.max(lowX, mapWidth - width - margin);
  const highY = Math.max(lowY, mapHeight - height - margin);

  let best = { x: lowX, y: lowY };
  let bestScore = Infinity;
  let bestFree = false;
  for (let y = lowY; y <= highY; y++) {
    for (let x = lowX; x <= highX; x++) {
      const free = !taken.some((box) => overlaps(box, { x, y, width, height }));
      const score = total(x, y);
      // A free position beats any overlapping one, however calm.
      if ((free && !bestFree) || (free === bestFree && score < bestScore)) {
        best = { x, y };
        bestScore = score;
        bestFree = free;
      }
    }
  }
  return best;
}
