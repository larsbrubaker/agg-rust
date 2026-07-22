// Reproduction test for the lion_outline "Use Scanline Rasterizer" on-canvas
// checkbox bug.
//
// The WASM side renders a "Use Scanline Rasterizer" CboxCtrl at AGG coords
// (160,5) (demo/wasm/src/render/compositing.rs:1475), but the TS demo
// registered only the Width slider in its canvasControls array — no checkbox
// descriptor. As a result, clicking the on-canvas checkbox did nothing (the
// sidebar checkbox worked, and dragging the on-canvas slider worked). This is
// the same bug class as line_patterns_clip's "Accurate Joins" checkbox.
//
// This test drives the REAL demo `init()` from src/demos/lion_outline.ts inside
// a happy-dom DOM, mocking only the WASM boundary (src/wasm.ts) so we can
// capture the params handed to the renderer. It asserts that a click at the
// on-canvas checkbox's displayed position toggles the use-scanline param.

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

const W = 512, H = 512;

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

test('lion_outline: clicking the on-canvas Use Scanline Rasterizer checkbox toggles the useScanline param', async () => {
  const { init } = await import('../src/demos/lion_outline.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  // params = [angle, scale, skewX, skewY, lineWidth, useScanline]; starts 0.
  expect(lastParams[5]).toBe(0);

  // The WASM CboxCtrl is at AGG (160,5). Click inside its hit region.
  // canvasPos maps clientY -> H - clientY (AGG bottom-left origin), so to click
  // AGG (165, 10) we use clientX = 165, clientY = H - 10 = 502.
  const clickX = 165, aggY = 10;
  canvas.dispatchEvent(pointer('pointerdown', clickX, H - aggY));
  canvas.dispatchEvent(pointer('pointerup', clickX, H - aggY));

  expect(lastParams[5]).toBe(1);

  // Clicking again toggles it back off.
  canvas.dispatchEvent(pointer('pointerdown', clickX, H - aggY));
  canvas.dispatchEvent(pointer('pointerup', clickX, H - aggY));

  expect(lastParams[5]).toBe(0);
});
