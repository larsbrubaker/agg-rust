// Reproduction test for the aa_demo "drag the triangle shape" bug.
//
// The C++ original (cpp-references/agg-src/examples/aa_demo.cpp) lets you drag
// the whole triangle by clicking inside it (on_mouse_button_down ->
// point_in_triangle -> m_idx = 3, on_mouse_move moves all three vertices).
// The web port was missing this: clicking inside the triangle did nothing.
//
// This test drives the REAL demo `init()` from src/demos/aa_demo.ts inside a
// happy-dom DOM, mocking only the WASM boundary (src/wasm.ts) so we can capture
// the triangle vertices that get handed to the renderer. It asserts that a
// left-drag starting INSIDE the triangle translates all three vertices.

import { test, expect, mock, beforeAll } from 'bun:test';
import { Window } from 'happy-dom';

// --- Capture the params (vertices) handed to the renderer ---
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

const W = 600, H = 400;

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
  // happy-dom has no canvas raster backend; provide a no-op 2D context so the
  // demo's draw() (renderToCanvas) completes without throwing.
  (win.HTMLCanvasElement.prototype as any).getContext = () => ({ putImageData: () => {} });
});

function prepCanvas(canvas: any) {
  // happy-dom has no layout engine: force a deterministic 1:1 rect.
  canvas.getBoundingClientRect = () => ({ left: 0, top: 0, width: W, height: H, right: W, bottom: H, x: 0, y: 0 });
  // pointer capture is a no-op in this environment.
  canvas.setPointerCapture = () => {};
  canvas.releasePointerCapture = () => {};
  if (!canvas.getContext) canvas.getContext = () => null;
}

function pointer(type: string, clientX: number, clientY: number) {
  const g = globalThis as any;
  const Ctor = g.PointerEvent;
  return new Ctor(type, { clientX, clientY, button: 0, buttons: 1, pointerId: 1, bubbles: true, cancelable: true });
}

test('aa_demo: dragging inside the triangle moves the whole shape (matches C++ point_in_triangle)', async () => {
  const { init } = await import('../src/demos/aa_demo.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  // Initial render captured vertices [57,100, 369,170, 143,310, pixelSize].
  const before = lastParams.slice(0, 6);
  expect(before).toEqual([57, 100, 369, 170, 143, 310]);

  // Triangle centroid in AGG space ~ (190, 193). canvasPos maps clientY -> H-clientY,
  // so to click AGG (190,193) we use clientY = H-193 = 207.
  const downX = 190, downClientY = H - 193; // 207
  canvas.dispatchEvent(pointer('pointerdown', downX, downClientY));
  // Drag +40px in x (AGG x increases with clientX).
  canvas.dispatchEvent(pointer('pointermove', downX + 40, downClientY));
  canvas.dispatchEvent(pointer('pointerup', downX + 40, downClientY));

  const after = lastParams.slice(0, 6);
  // All three vertices should have translated by ~+40 in x, unchanged in y.
  expect(after[0]).toBeCloseTo(before[0] + 40, 3); // x0
  expect(after[2]).toBeCloseTo(before[2] + 40, 3); // x1
  expect(after[4]).toBeCloseTo(before[4] + 40, 3); // x2
  expect(after[1]).toBeCloseTo(before[1], 3);      // y0 unchanged
});

test('aa_demo: dragging a single corner vertex still moves only that vertex', async () => {
  const { init } = await import('../src/demos/aa_demo.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  const before = lastParams.slice(0, 6);
  expect(before).toEqual([57, 100, 369, 170, 143, 310]);

  // Grab vertex[0] at AGG (57,100): clientX=57, clientY = H-100 = 300 (within threshold 10).
  canvas.dispatchEvent(pointer('pointerdown', 57, H - 100));
  canvas.dispatchEvent(pointer('pointermove', 57 + 30, H - 100));
  canvas.dispatchEvent(pointer('pointerup', 57 + 30, H - 100));

  const after = lastParams.slice(0, 6);
  expect(after[0]).toBeCloseTo(before[0] + 30, 3); // x0 moved
  expect(after[1]).toBeCloseTo(before[1], 3);      // y0 unchanged
  expect(after[2]).toBeCloseTo(before[2], 3);      // x1 unchanged
  expect(after[4]).toBeCloseTo(before[4], 3);      // x2 unchanged
});
