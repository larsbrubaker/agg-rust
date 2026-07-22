// Regression guard for the canvas-controls event-suppression contract, in the
// REGISTRATION-ORDER stress case.
//
// lion.ts registers setupRotateScale (src/mouse-helpers.ts, BUBBLE phase,
// pointermove driven purely by e.buttons) at line 28, BEFORE setupCanvasControls
// (CAPTURE phase) at line 39. Per DOM dispatch, the capture pass at the target
// runs first and sets the stop-propagation flag when a registered on-canvas
// control is hit, so the bubbling-pass invoke on the SAME canvas element returns
// early — the rotate/scale handler never sees the event, regardless of the
// (deliberately adversarial) registration order. (Verified empirically in both
// happy-dom v20 and Chromium.)
//
// This test pins that contract: dragging the on-canvas Alpha slider must change
// alpha but leave rotation/scale untouched. It fails loudly if a future refactor
// of canvas-controls.ts moves its listeners to the bubble phase or drops the
// stopPropagation()+preventDefault() calls.

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

test('lion: dragging the on-canvas Alpha slider changes alpha but leaves rotation/scale untouched', async () => {
  const { init } = await import('../src/demos/lion.ts');
  const container = document.createElement('div');
  document.body.appendChild(container);

  init(container);
  const canvas = document.getElementById('demo-canvas') as any;
  prepCanvas(canvas);

  // params = [angle, scale, skewX, skewY, alpha]; starts [0,1,0,0,26].
  expect(lastParams[0]).toBe(0);
  expect(lastParams[1]).toBe(1);
  expect(lastParams[4]).toBe(26);

  // Alpha slider bounds: x1=5,y1=5,x2=507,y2=12, min=0,max=255. Drag inside it.
  // AGG (100, 8) -> clientX=100, clientY = H - 8 = 392.
  const aggY = 8;
  canvas.dispatchEvent(pointer('pointerdown', 100, H - aggY));
  canvas.dispatchEvent(pointer('pointermove', 200, H - aggY));
  canvas.dispatchEvent(pointer('pointerup', 200, H - aggY));

  // The slider changed alpha...
  expect(lastParams[4]).not.toBe(26);
  // ...but setupRotateScale must NOT have fired. If the capture-phase suppression
  // contract regressed, handlePointer would run and set
  // angle=atan2(-192,-156)≈-2.26, scale≈2.47.
  expect(lastParams[0]).toBe(0);
  expect(lastParams[1]).toBe(1);
});
