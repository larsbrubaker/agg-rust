// Regression guard for the canvas-controls event-suppression contract.
//
// setupCanvasControls (src/canvas-controls.ts) attaches CAPTURE-phase pointer
// listeners on the canvas and calls stopPropagation()+preventDefault() when a
// registered on-canvas control (slider/checkbox/…) is hit. lion_outline also
// registers its OWN pointerdown/pointermove handlers on the SAME canvas element
// (bubble phase) that rotate/scale the lion from any pointer drag.
//
// Per DOM dispatch, the capture pass at the target runs first and sets the
// stop-propagation flag, so the bubbling-pass invoke on the SAME element returns
// early — the demo's bubble-phase drag handlers never see the event, regardless
// of listener registration order. (Verified empirically in both happy-dom v20
// and Chromium.) These tests pin that contract so a future refactor of
// canvas-controls.ts — e.g. moving its listeners to the bubble phase or dropping
// the stopPropagation calls — fails loudly.
//
// This drives the REAL demo init() from src/demos/lion_outline.ts inside a
// happy-dom DOM, mocking only the WASM boundary (src/wasm.ts) to capture the
// params handed to the renderer.

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

test('lion_outline: clicking the on-canvas checkbox does not disturb the rotation/scale drag state', async () => {
  const { init } = await import('../src/demos/lion_outline.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  // params = [angle, scale, skewX, skewY, lineWidth, useScanline]; starts [0,1,0,0,1,0].
  expect(lastParams[0]).toBe(0);
  expect(lastParams[1]).toBe(1);
  expect(lastParams[5]).toBe(0);

  // The WASM CboxCtrl is at AGG (160,5). Click inside its hit region.
  // canvasPos maps clientY -> H - clientY (AGG bottom-left origin), so to click
  // AGG (165, 10) we use clientX = 165, clientY = H - 10 = 502.
  const clickX = 165, aggY = 10;
  canvas.dispatchEvent(pointer('pointerdown', clickX, H - aggY));
  canvas.dispatchEvent(pointer('pointerup', clickX, H - aggY));

  // The checkbox toggled...
  expect(lastParams[5]).toBe(1);
  // ...but the demo's rotate/scale drag handler must NOT have fired. If the
  // capture-phase suppression contract regressed, updateTransform would run and
  // set angle=atan2(246,-91)≈1.93, scale≈2.62.
  expect(lastParams[0]).toBe(0);
  expect(lastParams[1]).toBe(1);
});

test('lion_outline: dragging the on-canvas Width slider does not disturb rotation/scale', async () => {
  const { init } = await import('../src/demos/lion_outline.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  // params = [angle, scale, skewX, skewY, lineWidth, useScanline]; starts [0,1,0,0,1,0].
  expect(lastParams[0]).toBe(0);
  expect(lastParams[1]).toBe(1);
  expect(lastParams[4]).toBe(1);

  // Width slider bounds: x1=5,y1=5,x2=150,y2=12. Click/drag inside it.
  // AGG (77, 8) -> clientX=77, clientY = H - 8 = 504.
  const aggY = 8;
  canvas.dispatchEvent(pointer('pointerdown', 77, H - aggY));
  canvas.dispatchEvent(pointer('pointermove', 120, H - aggY));
  canvas.dispatchEvent(pointer('pointerup', 120, H - aggY));

  // The slider changed the line width...
  expect(lastParams[4]).not.toBe(1);
  // ...but the demo's rotate/scale drag handler must NOT have fired.
  expect(lastParams[0]).toBe(0);
  expect(lastParams[1]).toBe(1);
});
