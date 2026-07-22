// Reproduction test for the line_patterns "handles can't be grabbed" bug.
//
// The C++ original (cpp-references/agg-src/examples/line_patterns.cpp) runs with
// flip_y = true and stores bezier control points in AGG bottom-left coords
// (e.g. m_curve1.curve(64, 19, ...)). The web port passes those same AGG coords
// to the WASM renderer, which draws flip_y like C++, and renderToCanvas mirrors
// the canvas with CSS scaleY(-1). So a handle stored at AGG y=Y is DISPLAYED at
// screen y = H - Y.
//
// The demo's bespoke hit-test compared the raw top-origin screen y against the
// stored AGG y, so the handles were vertically mirrored from where they could be
// grabbed — clicking a visible handle did nothing.
//
// This test drives the REAL demo init() under happy-dom, mocking only the WASM
// boundary (src/wasm.ts) to capture the params handed to the renderer. It clicks
// at a handle's DISPLAYED position and asserts the handle moves.

import { test, expect, mock, beforeAll } from 'bun:test';
import { Window } from 'happy-dom';

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

const W = 500, H = 450;

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
  canvas.getBoundingClientRect = () => ({ left: 0, top: 0, width: W, height: H, right: W, bottom: H, x: 0, y: 0 });
  canvas.setPointerCapture = () => {};
  canvas.releasePointerCapture = () => {};
  if (!canvas.getContext) canvas.getContext = () => null;
}

function pointer(type: string, clientX: number, clientY: number) {
  const g = globalThis as any;
  const Ctor = g.PointerEvent;
  return new Ctor(type, { clientX, clientY, button: 0, buttons: 1, pointerId: 1, bubbles: true, cancelable: true });
}

test('line_patterns: dragging a control point at its DISPLAYED position moves that point', async () => {
  const { init } = await import('../src/demos/line_patterns.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  // params = [scaleX, startX, ...72 point coords]. First control point is AGG (64, 19).
  const beforeX = lastParams[2];
  const beforeY = lastParams[3];
  expect(beforeX).toBe(64);
  expect(beforeY).toBe(19);

  // The handle stored at AGG (64, 19) DISPLAYS at screen (64, H - 19 = 431)
  // because of the CSS scaleY(-1) flip. Click there.
  const downX = 64, downClientY = H - 19; // 431
  canvas.dispatchEvent(pointer('pointerdown', downX, downClientY));
  // Drag +30px in x, keep displayed y the same.
  canvas.dispatchEvent(pointer('pointermove', downX + 30, downClientY));
  canvas.dispatchEvent(pointer('pointerup', downX + 30, downClientY));

  const afterX = lastParams[2];
  const afterY = lastParams[3];
  // The grabbed control point should follow the cursor: x moves +30, AGG y unchanged.
  expect(afterX).toBeCloseTo(beforeX + 30, 3);
  expect(afterY).toBeCloseTo(beforeY, 3);
});
