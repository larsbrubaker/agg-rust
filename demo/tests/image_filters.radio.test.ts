// Reproduction test for the image_filters on-canvas radio (filter selection) bug.
//
// The WASM side renders the 17-item filter RboxCtrl with text_size(6.0)
// (images.rs image_filters_demo, faithfully matching C++ image_filters.cpp:119
// `m_filters.text_size(6.0)`). rbox_ctrl item spacing is dy = 2*text_height, so
// the TRUE geometry is dy=12: circle centers at
//   cx   = x1 + 1 + dy/1.3        (= 10.23 for x1=0)
//   cy_i = y1 + 1 + dy*i + dy/1.3 (= 10.23 + 12*i for y1=0)
// with hit radius text_height/1.5 = 4.
//
// The on-canvas hit test in canvas-controls.ts derives dy/cx/cy/radius from the
// descriptor's textHeight (default 9.0 -> dy=18). The image_filters descriptor
// never passed textHeight, so the hit test assumed 18px spacing while the WASM
// draws items 12px apart. A click on the visual position of a filter therefore
// selected an item ~5 rows higher — clicking "sinc" (item 14) landed on
// "quadric" (item 9). This test drives the REAL demo init() and pins the fix:
// clicks at TRUE (text_size 6) geometry must select the item actually drawn there.
//
// It mocks only the WASM boundary (src/wasm.ts) to capture the params handed to
// the renderer. params = [filterIdx, stepDeg, normalize, radius, numSteps,
// kpixSec, incremental]; filterIdx is params[0], initial 1.

import { test, expect, mock, beforeAll } from 'bun:test';
import { Window } from 'happy-dom';

// --- Capture the params handed to the renderer ---
let lastParams: number[] = [];
mock.module('../src/wasm.ts', () => ({
  initWasm: async () => {},
  renderDemo: (_name: string, w: number, h: number, params: number[] | Float64Array) => {
    lastParams = Array.from(params);
    return new Uint8Array(w * h * 4);
  },
  flashPickVertex: () => -1,
  flashScreenToShape: (_a: any, _b: any, _c: any, _d: any, x: number, y: number) => [x, y],
  gouraudMeshPickVertex: () => -1,
  getVersion: () => 'test',
}));

// image_filters renders at 430x340.
const W = 430, H = 340;

beforeAll(() => {
  const win = new Window({ width: W, height: H });
  const g = globalThis as any;
  g.window = win;
  g.document = win.document;
  g.HTMLElement = win.HTMLElement;
  g.HTMLCanvasElement = win.HTMLCanvasElement;
  g.HTMLInputElement = win.HTMLInputElement;
  g.Event = win.Event;
  g.PointerEvent = win.PointerEvent ?? win.MouseEvent;
  g.MouseEvent = win.MouseEvent;
  g.ImageData = win.ImageData;
  g.getComputedStyle = win.getComputedStyle.bind(win);
  (win.HTMLCanvasElement.prototype as any).getContext = () => ({ putImageData: () => {} });
});

function prepCanvas(canvas: any) {
  // Report the layout rect equal to the AGG buffer size so pointer coords map 1:1.
  canvas.getBoundingClientRect = () => ({
    left: 0, top: 0, width: canvas.width, height: canvas.height,
    right: canvas.width, bottom: canvas.height, x: 0, y: 0,
  });
  canvas.setPointerCapture = () => {};
  canvas.releasePointerCapture = () => {};
  if (!canvas.getContext) canvas.getContext = () => null;
}

function pointer(type: string, clientX: number, clientY: number) {
  const g = globalThis as any;
  const Ctor = g.PointerEvent;
  return new Ctor(type, { clientX, clientY, button: 0, buttons: 1, pointerId: 1, bubbles: true, cancelable: true });
}

// TRUE radio circle geometry, mirroring rbox_ctrl with the WASM's text_size(6.0):
// text_height=6 => dy=12, cx = (x1+1)+dy/1.3, cy_i = (y1+1)+dy*i+dy/1.3, radius=6/1.5=4.
// (These positions are where the WASM actually draws the item circles. The old
// test used text_height=9 => dy=18, which matched neither the WASM render nor
// the C++ original, so it passed while the real UI misselected filters.)
const X1 = 0, Y1 = 0, DY = 6.0 * 2.0;
const CX = X1 + 1.0 + DY / 1.3;
function cyForItem(i: number): number {
  return Y1 + 1.0 + DY * i + DY / 1.3;
}

// Filter list order (matches images.rs filter_names and the demo's filterNames):
// 0 simple, 1 bilinear, ..., 9 quadric, ..., 14 sinc, 15 lanczos, 16 blackman.
const SINC = 14;
const BLACKMAN = 16;

test('image_filters: clicking on-canvas radio "sinc" (item 14) selects filter 14, not quadric (9)', async () => {
  const { init } = await import('../src/demos/image_filters.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  // Initial filter index is 1.
  expect(lastParams[0]).toBe(1);

  // Click the position where the WASM draws "sinc" (item 14): cx≈10.23, cy≈178.23.
  // Map AGG y to clientY via clientY = H - aggY (bottom-left origin).
  const aggX = CX;
  const aggY = cyForItem(SINC);
  canvas.dispatchEvent(pointer('pointerdown', aggX, H - aggY));
  canvas.dispatchEvent(pointer('pointerup', aggX, H - aggY));

  // With the pre-fix default-9 geometry the hit test resolves this point to
  // item 9 (quadric); the fix (textHeight:6) makes it resolve to 14 (sinc).
  expect(lastParams[0]).toBe(SINC);
});

test('image_filters: clicking on-canvas radio "blackman" (item 16, the lowest) selects filter 16', async () => {
  const { init } = await import('../src/demos/image_filters.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  const aggY = cyForItem(BLACKMAN); // cy≈202.23
  canvas.dispatchEvent(pointer('pointerdown', CX, H - aggY));
  canvas.dispatchEvent(pointer('pointerup', CX, H - aggY));

  expect(lastParams[0]).toBe(BLACKMAN);
});

test('image_filters: clicking on-canvas radio item 1 (bilinear) still selects filter 1 (regression guard)', async () => {
  const { init } = await import('../src/demos/image_filters.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  // First select a different filter (sinc / item 14) so a change back to 1 is observable.
  canvas.dispatchEvent(pointer('pointerdown', CX, H - cyForItem(SINC)));
  canvas.dispatchEvent(pointer('pointerup', CX, H - cyForItem(SINC)));
  expect(lastParams[0]).toBe(SINC);

  // Item 1's circle center at TRUE geometry (cx≈10.23, cy≈22.23).
  const aggY = cyForItem(1);
  canvas.dispatchEvent(pointer('pointerdown', CX, H - aggY));
  canvas.dispatchEvent(pointer('pointerup', CX, H - aggY));

  expect(lastParams[0]).toBe(1);
});
