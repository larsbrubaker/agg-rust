// Determine WASM render buffer orientation (top-origin vs bottom-origin)
// by rendering demos and locating known features in the returned RGBA buffer.
import { readFileSync } from 'fs';
import { initSync, render_demo } from '../public/pkg/agg_wasm.js';

const bytes = readFileSync(new URL('../public/pkg/agg_wasm_bg.wasm', import.meta.url).pathname.replace(/^\//, ''));
initSync({ module: bytes });

function render(name: string, w: number, h: number, params: number[]): Uint8Array {
  return render_demo(name, w, h, new Float64Array(params));
}

// aa_demo: default vertices (57,100),(369,170),(143,310), pixel_size 32.
// Find vertical distribution of BLACK-ish pixels (the enlarged triangle is black).
{
  const W = 600, H = 400;
  const buf = render('aa_demo', W, H, [57, 100, 369, 170, 143, 310, 32]);
  let minRow = H, maxRow = -1, blackCount = 0;
  const rowBlack = new Array(H).fill(0);
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const i = (y * W + x) * 4;
      const r = buf[i], g = buf[i + 1], b = buf[i + 2];
      if (r < 60 && g < 60 && b < 60) { rowBlack[y]++; blackCount++; if (y < minRow) minRow = y; if (y > maxRow) maxRow = y; }
    }
  }
  // weighted centroid row
  let sum = 0, wsum = 0;
  for (let y = 0; y < H; y++) { sum += rowBlack[y]; wsum += rowBlack[y] * y; }
  const centroidRow = wsum / sum;
  console.log(`aa_demo: black rows [${minRow}..${maxRow}] centroidRow=${centroidRow.toFixed(1)} (blackPx=${blackCount})`);
  console.log('  AGG-space triangle y in [100..310], centroid~193.');
  console.log(`  If buffer TOP-origin: expect black centroid ~193. If BOTTOM-origin: expect ~${(400-193).toFixed(0)}.`);
}

// gradients: highlight center (cx,cy)=(350,280). Find brightest region row near x=350.
{
  const W = 512, H = 400;
  const splineR = [0,1, 0.2,0.8, 0.4,0.6, 0.6,0.4, 0.8,0.2, 1,0];
  const splineG = splineR.slice();
  const splineB = splineR.slice();
  const splineA = [0,1, 0.2,1, 0.4,1, 0.6,1, 0.8,1, 1,1];
  const params = [350,280, 0, 1, 0, 1,1, 1,1,1,1, ...splineR, ...splineG, ...splineB, ...splineA];
  const buf = render('gradients', W, H, params);
  // Column band around x=350, find row of max brightness (sum RGB)
  let bestRow = -1, bestVal = -1;
  for (let y = 0; y < H; y++) {
    let s = 0, n = 0;
    for (let x = 340; x <= 360; x++) { const i = (y * W + x) * 4; s += buf[i] + buf[i+1] + buf[i+2]; n++; }
    const avg = s / n;
    if (avg > bestVal) { bestVal = avg; bestRow = y; }
  }
  console.log(`\ngradients: brightest row near x=350 is row=${bestRow} (avg=${bestVal.toFixed(0)})`);
  console.log(`  AGG-space center cy=280. If TOP-origin: bright ~280. If BOTTOM-origin: ~${400-280}.`);
}
