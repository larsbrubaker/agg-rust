// Reproduction test for the gradients demo "drag the highlight spot" bug.
//
// The C++ original (cpp-references/agg-src/examples/gradients.cpp) lets you drag
// the gradient center (the highlight sphere) by left-clicking inside the r=110
// sphere: on_mouse_button_down arms the move, on_mouse_move translates
// m_center_x / m_center_y. The web port lost this: the sphere never engaged
// because a per-region raw-vs-flipped coordinate heuristic mis-classified the
// click. The sphere's DISPLAYED (CSS-flipped) position happens to fall inside a
// spline-box region, so the handler switched to raw coordinates and the
// inSphere() test — which is expressed in demo/agg coordinates — failed.
//
// This test drives the REAL demo init() from src/demos/gradients.ts inside a
// happy-dom DOM, mocking only the WASM boundary (src/wasm.ts) so we can capture
// the center params handed to the renderer. It asserts that a left-drag starting
// on the sphere's DISPLAYED position translates the gradient center, and (as a
// regression guard) that dragging a spline control point still works.

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

// gradients.ts renders a 512x400 buffer.
const W = 512, H = 400;

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
  // happy-dom has no layout engine: force a deterministic 1:1 rect.
  canvas.getBoundingClientRect = () => ({ left: 0, top: 0, width: W, height: H, right: W, bottom: H, x: 0, y: 0 });
  canvas.setPointerCapture = () => {};
  canvas.releasePointerCapture = () => {};
  if (!canvas.getContext) canvas.getContext = () => null;
}

function pointer(type: string, clientX: number, clientY: number, button = 0) {
  const g = globalThis as any;
  const Ctor = g.PointerEvent;
  return new Ctor(type, { clientX, clientY, button, buttons: 1, pointerId: 1, bubbles: true, cancelable: true });
}

test('gradients: left-dragging the highlight sphere translates the gradient center (matches C++ m_center_x/y)', async () => {
  const { init } = await import('../src/demos/gradients.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  // Initial center: params[0]=cx=350, params[1]=cy=280.
  const beforeCx = lastParams[0];
  const beforeCy = lastParams[1];
  expect(beforeCx).toBe(350);
  expect(beforeCy).toBe(280);

  // The sphere center is at demo/agg (350, 280). canvasPos maps clientY -> H-clientY,
  // so its DISPLAYED position is clientY = H - 280 = 120. Click there and drag +40 in x.
  const downX = 350, downClientY = H - 280; // 120
  canvas.dispatchEvent(pointer('pointerdown', downX, downClientY));
  canvas.dispatchEvent(pointer('pointermove', downX + 40, downClientY));
  canvas.dispatchEvent(pointer('pointerup', downX + 40, downClientY));

  const afterCx = lastParams[0];
  const afterCy = lastParams[1];
  expect(afterCx).toBeCloseTo(beforeCx + 40, 3); // center moved +40 in x
  expect(afterCy).toBeCloseTo(beforeCy, 3);      // center unchanged in y
});

test('gradients: dragging a spline control point still updates that point (regression guard)', async () => {
  const { init } = await import('../src/demos/gradients.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  // splineR flattened params start at index 11: [x0,y0,x1,y1,...]. Point idx=2 y is at index 16.
  const R_BASE = 11;
  const idx = 2;
  const yParam = R_BASE + idx * 2 + 1; // 16
  const beforeY = lastParams[yParam];
  expect(beforeY).toBeCloseTo(0.6, 6); // default splineR point 2: y = 1 - 2/5

  // splineR box is (210,10,460,45); splinePointToCanvas(0,2) => agg (~310.2, ~30.8).
  const bw = 1;
  const xs1 = 210 + bw, ys1 = 10 + bw, xs2 = 460 - bw, ys2 = 45 - bw;
  const spX = xs1 + (xs2 - xs1) * (idx / 5);
  const spY = ys1 + (ys2 - ys1) * (1 - idx / 5);

  // Grab the point (agg coords -> clientY = H - aggY), then drag it down toward y=22 (agg).
  canvas.dispatchEvent(pointer('pointerdown', spX, H - spY));
  const targetAggY = 22;
  canvas.dispatchEvent(pointer('pointermove', spX, H - targetAggY));
  canvas.dispatchEvent(pointer('pointerup', spX, H - targetAggY));

  const afterY = lastParams[yParam];
  const expectedY = (targetAggY - ys1) / (ys2 - ys1);
  expect(afterY).toBeCloseTo(expectedY, 3);
  expect(afterY).not.toBeCloseTo(beforeY, 3);
});
