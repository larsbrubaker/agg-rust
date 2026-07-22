// Reproduction test for the image_filters on-canvas radio (filter selection) bug.
//
// The WASM side renders a 17-item RboxCtrl for filter selection, registered in
// the TS demo as a radio control with bounding box (0,0,110,210) — matching the
// WASM RboxCtrl::new(0,0,110,210). But the 17 radio item circles overflow that
// box: with textHeight=9, dy=18, each item's circle center is
//   cy_i = y1+1 + dy*i + dy/1.3
// so item 16 sits at cy≈302.8, well past the box's y2=210. The old hitTest gated
// EVERY control on its bounding box before running the type-specific circle test,
// so clicks on items >=11 (cy>=212.8 > 210) failed the gate and were ignored.
//
// C++ AGG's ctrl_container::on_mouse_button_down forwards clicks to each ctrl
// with NO bounding-box gate; rbox_ctrl decides purely by the per-item circle
// test, so these clicks work in the C++ demo. This test pins that behavior.
//
// It drives the REAL demo init() from src/demos/image_filters.ts inside a
// happy-dom DOM, mocking only the WASM boundary (src/wasm.ts) to capture the
// params handed to the renderer. params = [filterIdx, stepDeg, normalize,
// radius, numSteps, kpixSec, incremental]; filterIdx is params[0], initial 1.

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

// Radio circle geometry, mirroring rbox_ctrl / canvas-controls circle math.
// textHeight defaults to 9.0 => dy=18, cx = (x1+1)+dy/1.3, cy_i = (y1+1)+dy*i+dy/1.3.
const X1 = 0, Y1 = 0, DY = 9.0 * 2.0;
const CX = X1 + 1.0 + DY / 1.3;
function cyForItem(i: number): number {
  return Y1 + 1.0 + DY * i + DY / 1.3;
}

test('image_filters: clicking on-canvas radio item 14 (overflowing the rbox) selects filter 14', async () => {
  const { init } = await import('../src/demos/image_filters.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  // Initial filter index is 1.
  expect(lastParams[0]).toBe(1);

  // Item 14's circle center in AGG coords (cx≈14.85, cy≈266.85) — this circle
  // lies BELOW the registered rbox box (y2=210), so the old box-gated hitTest
  // ignored it. Map AGG y to clientY via clientY = H - aggY (bottom-left origin).
  const aggX = CX;            // ≈ 14.846
  const aggY = cyForItem(14); // ≈ 266.846
  canvas.dispatchEvent(pointer('pointerdown', aggX, H - aggY));
  canvas.dispatchEvent(pointer('pointerup', aggX, H - aggY));

  expect(lastParams[0]).toBe(14);
});

test('image_filters: clicking on-canvas radio item 1 (inside the rbox) still selects filter 1 (regression guard)', async () => {
  const { init } = await import('../src/demos/image_filters.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  // First select a different filter (item 14) so a change back to 1 is observable.
  canvas.dispatchEvent(pointer('pointerdown', CX, H - cyForItem(14)));
  canvas.dispatchEvent(pointer('pointerup', CX, H - cyForItem(14)));
  expect(lastParams[0]).toBe(14);

  // Item 1's circle center (cx≈14.85, cy≈32.85) is inside the rbox box and was
  // always clickable — this guards the already-working region against regression.
  const aggY = cyForItem(1); // ≈ 32.846
  canvas.dispatchEvent(pointer('pointerdown', CX, H - aggY));
  canvas.dispatchEvent(pointer('pointerup', CX, H - aggY));

  expect(lastParams[0]).toBe(1);
});
