import { createDemoLayout, addSlider, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

function pointInTriangle(
  ax: number, ay: number,
  bx: number, by: number,
  cx: number, cy: number,
  px: number, py: number,
): boolean {
  const sign = (x1: number, y1: number, x2: number, y2: number, x3: number, y3: number) =>
    (x1 - x3) * (y2 - y3) - (x2 - x3) * (y1 - y3);
  const d1 = sign(px, py, ax, ay, bx, by);
  const d2 = sign(px, py, bx, by, cx, cy);
  const d3 = sign(px, py, cx, cy, ax, ay);
  const hasNeg = d1 < 0 || d2 < 0 || d3 < 0;
  const hasPos = d1 > 0 || d2 > 0 || d3 > 0;
  return !(hasNeg && hasPos);
}

function canvasPos(canvas: HTMLCanvasElement, e: PointerEvent): { x: number; y: number } {
  const rect = canvas.getBoundingClientRect();
  const scaleX = canvas.width / rect.width;
  const scaleY = canvas.height / rect.height;
  return {
    x: (e.clientX - rect.left) * scaleX,
    y: canvas.height - (e.clientY - rect.top) * scaleY,
  };
}

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Rasterizers',
    'Aliased vs anti-aliased rasterization with gamma and alpha controls.',
  );

  const W = 500, H = 330;

  const vertices: Vertex[] = [
    { x: 220, y: 60 },
    { x: 489, y: 170 },
    { x: 263, y: 310 },
  ];

  let gammaVal = 0.5;
  let alpha = 1.0;
  let testPerformance = false;

  function draw() {
    renderToCanvas({
      demoName: 'rasterizers',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        gammaVal, alpha, testPerformance ? 1.0 : 0.0,
      ],
      timeDisplay: timeEl,
    });
  }

  // Match C++ rasterizers.cpp drag semantics:
  // - pick vertex near either right triangle (x) or left triangle (x-200)
  // - drag-all when clicking inside either triangle
  let dragIdx = -1;
  let dx = 0;
  let dy = 0;
  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const pos = canvasPos(canvas, e);
    const threshold = 20;
    for (let i = 0; i < 3; i++) {
      const dRight = Math.hypot(pos.x - vertices[i].x, pos.y - vertices[i].y);
      const dLeft = Math.hypot(pos.x - (vertices[i].x - 200), pos.y - vertices[i].y);
      if (dRight < threshold || dLeft < threshold) {
        dx = pos.x - vertices[i].x;
        dy = pos.y - vertices[i].y;
        dragIdx = i;
        canvas.setPointerCapture(e.pointerId);
        return;
      }
    }
    const inRight = pointInTriangle(
      vertices[0].x, vertices[0].y,
      vertices[1].x, vertices[1].y,
      vertices[2].x, vertices[2].y,
      pos.x, pos.y,
    );
    const inLeft = pointInTriangle(
      vertices[0].x - 200, vertices[0].y,
      vertices[1].x - 200, vertices[1].y,
      vertices[2].x - 200, vertices[2].y,
      pos.x, pos.y,
    );
    if (inRight || inLeft) {
      dx = pos.x - vertices[0].x;
      dy = pos.y - vertices[0].y;
      dragIdx = 3;
      canvas.setPointerCapture(e.pointerId);
    }
  }
  function onPointerMove(e: PointerEvent) {
    if (dragIdx < 0 || (e.buttons & 1) === 0) return;
    const pos = canvasPos(canvas, e);
    if (dragIdx === 3) {
      const newX = pos.x - dx;
      const newY = pos.y - dy;
      const ddx = newX - vertices[0].x;
      const ddy = newY - vertices[0].y;
      for (const v of vertices) {
        v.x += ddx;
        v.y += ddy;
      }
      draw();
      return;
    }
    vertices[dragIdx].x = pos.x - dx;
    vertices[dragIdx].y = pos.y - dy;
    draw();
  }
  function onPointerUp() {
    dragIdx = -1;
  }
  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);

  const slGamma = addSlider(sidebar, 'Gamma', 0.0, 1.0, 0.5, 0.01, v => { gammaVal = v; draw(); });
  const slAlpha = addSlider(sidebar, 'Alpha', 0.0, 1.0, 1.0, 0.01, v => { alpha = v; draw(); });
  const cbTest = addCheckbox(sidebar, 'Test Performance', false, v => {
    // Keep C++ UX where test checkbox immediately clears after activation.
    testPerformance = v;
    draw();
    if (v) {
      testPerformance = false;
      cbTest.checked = false;
      draw();
    }
  });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 140, y1: 14, x2: 280, y2: 22, min: 0, max: 1, sidebarEl: slGamma, onChange: v => { gammaVal = v; draw(); } },
    { type: 'slider', x1: 290, y1: 14, x2: 490, y2: 22, min: 0, max: 1, sidebarEl: slAlpha, onChange: v => { alpha = v; draw(); } },
    { type: 'checkbox', x1: 140, y1: 30, x2: 270, y2: 46, sidebarEl: cbTest, onChange: v => {
      testPerformance = v;
      draw();
      if (v) {
        testPerformance = false;
        cbTest.checked = false;
        draw();
      }
    } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: 'bottom-left' });

  function onKeyDown(e: KeyboardEvent) {
    let ddx = 0;
    let ddy = 0;
    switch (e.key) {
      case 'ArrowLeft': ddx = -0.1; break;
      case 'ArrowRight': ddx = 0.1; break;
      case 'ArrowUp': ddy = 0.1; break;
      case 'ArrowDown': ddy = -0.1; break;
      default: return;
    }
    vertices[0].x += ddx;
    vertices[0].y += ddy;
    vertices[1].x += ddx;
    vertices[1].y += ddy;
    draw();
    e.preventDefault();
  }
  window.addEventListener('keydown', onKeyDown);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag vertices on either triangle or drag inside triangle to move all.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    cleanupCC();
    window.removeEventListener('keydown', onKeyDown);
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
  };
}
