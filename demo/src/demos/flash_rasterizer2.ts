import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { flashPickVertex, flashScreenToShape } from '../wasm.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container, 'Flash Rasterizer 2',
    'Multi-style shapes rendered with regular rasterizer — matching C++ flash_rasterizer2.cpp behavior.',
  );
  const W = 655, H = 520;
  type Mtx = [number, number, number, number, number, number]; // [sx, shy, shx, sy, tx, ty]
  const m: Mtx = [1, 0, 0, 1, 0, 0];
  let shapeIndex = 0;
  const editedVertices = new Map<number, { x: number; y: number }>();
  let pointerX = W * 0.5;
  let pointerY = H * 0.5;
  let dragVertex = -1;
  let dragUsesFlippedY = false;

  function mulInPlace(b: Mtx) {
    const a0 = m[0], a1 = m[1], a2 = m[2], a3 = m[3], a4 = m[4], a5 = m[5];
    m[0] = a0 * b[0] + a1 * b[2];
    m[2] = a2 * b[0] + a3 * b[2];
    m[4] = a4 * b[0] + a5 * b[2] + b[4];
    m[1] = a0 * b[1] + a1 * b[3];
    m[3] = a2 * b[1] + a3 * b[3];
    m[5] = a4 * b[1] + a5 * b[3] + b[5];
  }

  function translate(tx: number, ty: number): Mtx {
    return [1, 0, 0, 1, tx, ty];
  }

  function scale(s: number): Mtx {
    return [s, 0, 0, s, 0, 0];
  }

  function rotate(a: number): Mtx {
    const c = Math.cos(a);
    const s = Math.sin(a);
    return [c, s, -s, c, 0, 0];
  }

  function applyAroundPointer(op: Mtx) {
    mulInPlace(translate(-pointerX, -pointerY));
    mulInPlace(op);
    mulInPlace(translate(pointerX, pointerY));
  }

  function approxScale(): number {
    return Math.max(0.001, Math.hypot(m[0], m[2]));
  }

  function buildParams(): number[] {
    const edits: number[] = [];
    const sorted = [...editedVertices.entries()].sort((a, b) => a[0] - b[0]);
    for (const [idx, p] of sorted) edits.push(idx, p.x, p.y);
    // Keep slots [7..9] aligned with flash_rasterizer extended param format.
    return [
      shapeIndex,
      m[0], m[1], m[2], m[3], m[4], m[5],
      pointerX, pointerY, 0,
      ...edits,
    ];
  }

  function draw() {
    renderToCanvas({ demoName: 'flash_rasterizer2', canvas, width: W, height: H,
      params: buildParams(), timeDisplay: timeEl, flipY: false });
    shapeInfo.textContent = `Shape: ${shapeIndex}`;
  }

  function addButton(label: string, onClick: () => void): HTMLButtonElement {
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.textContent = label;
    btn.style.cssText = 'display:block;margin:4px 0;padding:6px 10px;cursor:pointer;font-size:12px;width:100%;';
    btn.addEventListener('click', onClick);
    sidebar.appendChild(btn);
    return btn;
  }

  const shapeInfo = document.createElement('div');
  shapeInfo.className = 'control-hint';
  shapeInfo.textContent = 'Shape: 0';
  sidebar.appendChild(shapeInfo);

  addButton('Next Shape (Space)', () => {
    shapeIndex += 1;
    editedVertices.clear();
    dragVertex = -1;
    draw();
  });
  addButton('Zoom In (+)', () => { applyAroundPointer(scale(1.1)); draw(); });
  addButton('Zoom Out (-)', () => { applyAroundPointer(scale(1 / 1.1)); draw(); });
  addButton('Rotate Left (\u2190)', () => { applyAroundPointer(rotate(-Math.PI / 20.0)); draw(); });
  addButton('Rotate Right (\u2192)', () => { applyAroundPointer(rotate(Math.PI / 20.0)); draw(); });
  addButton('Reset View', () => {
    m[0] = 1; m[1] = 0; m[2] = 0; m[3] = 1; m[4] = 0; m[5] = 0;
    draw();
  });

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Canvas: left-drag vertices. Arrows rotate around mouse, +/- zoom around mouse.';
  sidebar.appendChild(hint);

  function canvasPos(e: PointerEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    const sx = W / rect.width;
    const sy = H / rect.height;
    return {
      x: (e.clientX - rect.left) * sx,
      y: (e.clientY - rect.top) * sy,
    };
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === ' ') {
      shapeIndex += 1;
      editedVertices.clear();
      dragVertex = -1;
      draw();
      e.preventDefault();
      return;
    }
    if (e.key === '+' || e.key === '=' || e.code === 'NumpadAdd') {
      applyAroundPointer(scale(1.1));
      draw();
      e.preventDefault();
      return;
    }
    if (e.key === '-' || e.code === 'NumpadSubtract') {
      applyAroundPointer(scale(1 / 1.1));
      draw();
      e.preventDefault();
      return;
    }
    if (e.key === 'ArrowLeft') {
      applyAroundPointer(rotate(-Math.PI / 20.0));
      draw();
      e.preventDefault();
      return;
    }
    if (e.key === 'ArrowRight') {
      applyAroundPointer(rotate(Math.PI / 20.0));
      draw();
      e.preventDefault();
    }
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const p = canvasPos(e);
    pointerX = p.x;
    pointerY = p.y;
    const pickRadius = 10.0 / approxScale();
    let hit = flashPickVertex('flash_rasterizer2', W, H, buildParams(), p.x, p.y, pickRadius);
    dragUsesFlippedY = false;
    if (hit < 0) {
      const yFlip = H - p.y;
      hit = flashPickVertex('flash_rasterizer2', W, H, buildParams(), p.x, yFlip, pickRadius);
      if (hit >= 0) {
        dragUsesFlippedY = true;
      }
    }
    dragVertex = hit;
    canvas.setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function onPointerMove(e: PointerEvent) {
    const p = canvasPos(e);
    pointerX = p.x;
    pointerY = p.y;
    if (dragVertex >= 0 && (e.buttons & 1) !== 0) {
      const dragY = dragUsesFlippedY ? (H - p.y) : p.y;
      const [lx, ly] = flashScreenToShape('flash_rasterizer2', W, H, buildParams(), p.x, dragY);
      editedVertices.set(dragVertex, { x: lx, y: ly });
      draw();
      e.preventDefault();
      return;
    }
    if (dragVertex >= 0 && (e.buttons & 1) === 0) {
      dragVertex = -1;
      draw();
    }
  }

  function onPointerUp() {
    dragUsesFlippedY = false;
    if (dragVertex >= 0) {
      dragVertex = -1;
      draw();
    }
  }

  window.addEventListener('keydown', onKeyDown);
  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);
  draw();
  return () => {
    window.removeEventListener('keydown', onKeyDown);
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
  };
}
