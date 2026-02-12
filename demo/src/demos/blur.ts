import { createDemoLayout, addSlider, addRadioGroup, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Blur',
    'Stack blur and recursive blur on colored shapes — matching C++ blur.cpp.',
  );

  const W = 440, H = 330;
  let radius = 15.0;
  let method = 0;
  const methodLabels = ['Stack blur', 'Recursive blur', 'Channels'];
  let channelR = false;
  let channelG = true;
  let channelB = false;

  type Vertex = { x: number; y: number };
  type Quad = [Vertex, Vertex, Vertex, Vertex];
  const shadowQuad: Quad = [
    { x: 164.24, y: 96.28 },
    { x: 326.76, y: 96.28 },
    { x: 326.76, y: 284.16 },
    { x: 164.24, y: 284.16 },
  ];

  function draw() {
    renderToCanvas({
      demoName: 'blur',
      canvas, width: W, height: H,
      params: [
        radius,
        method,
        channelR ? 1 : 0,
        channelG ? 1 : 0,
        channelB ? 1 : 0,
        shadowQuad[0].x, shadowQuad[0].y,
        shadowQuad[1].x, shadowQuad[1].y,
        shadowQuad[2].x, shadowQuad[2].y,
        shadowQuad[3].x, shadowQuad[3].y,
      ],
      timeDisplay: timeEl,
    });
  }

  const slRadius = addSlider(sidebar, 'Blur Radius', 0, 40, 15, 0.01, v => { radius = v; draw(); });
  // C++ rbox renders first item at the bottom. Mirror that visual order in sidebar.
  const methodLabelsUi = [...methodLabels].reverse();
  const methodInputsUi = addRadioGroup(
    sidebar,
    'Method',
    methodLabelsUi,
    methodLabels.length - 1 - method,
    uiIndex => {
      method = methodLabels.length - 1 - uiIndex;
      draw();
    },
  );
  const methodInputs: HTMLInputElement[] = new Array(methodLabels.length);
  for (let uiIndex = 0; uiIndex < methodLabels.length; uiIndex++) {
    const logicalIndex = methodLabels.length - 1 - uiIndex;
    methodInputs[logicalIndex] = methodInputsUi[uiIndex];
  }
  const cbRed = addCheckbox(sidebar, 'Red', channelR, v => { channelR = v; draw(); });
  const cbGreen = addCheckbox(sidebar, 'Green', channelG, v => { channelG = v; draw(); });
  const cbBlue = addCheckbox(sidebar, 'Blue', channelB, v => { channelB = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 140, y1: 14, x2: 430, y2: 22, min: 0, max: 40, sidebarEl: slRadius, onChange: v => { radius = v; draw(); } },
    { type: 'radio', x1: 10, y1: 10, x2: 130, y2: 70, numItems: 3, sidebarEls: methodInputs, onChange: i => { method = i; draw(); } },
    { type: 'checkbox', x1: 10, y1: 80, x2: 95, y2: 92, sidebarEl: cbRed, onChange: v => { channelR = v; draw(); } },
    { type: 'checkbox', x1: 10, y1: 95, x2: 95, y2: 107, sidebarEl: cbGreen, onChange: v => { channelG = v; draw(); } },
    { type: 'checkbox', x1: 10, y1: 110, x2: 95, y2: 122, sidebarEl: cbBlue, onChange: v => { channelB = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: 'bottom-left' });

  function aggPos(e: PointerEvent): Vertex {
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: H - (e.clientY - rect.top) * scaleY,
    };
  }

  function inControlBounds(p: Vertex): boolean {
    for (const c of canvasControls) {
      const extra = (c.type === 'slider' || c.type === 'scale') ? (c.y2 - c.y1) / 2 : 0;
      if (
        p.x >= c.x1 - extra && p.x <= c.x2 + extra &&
        p.y >= c.y1 - extra && p.y <= c.y2 + extra
      ) {
        return true;
      }
    }
    return false;
  }

  function pointInPoly(p: Vertex, poly: Quad): boolean {
    let inside = false;
    let j = poly.length - 1;
    for (let i = 0; i < poly.length; i++) {
      const pi = poly[i];
      const pj = poly[j];
      const intersects = ((pi.y > p.y) !== (pj.y > p.y))
        && (p.x < (pj.x - pi.x) * (p.y - pi.y) / ((pj.y - pi.y) || 1e-12) + pi.x);
      if (intersects) inside = !inside;
      j = i;
    }
    return inside;
  }

  let drag:
    | { kind: 'vertex'; idx: number; dx: number; dy: number }
    | { kind: 'all'; lastX: number; lastY: number }
    | null = null;

  function nearestVertex(p: Vertex): { idx: number; d: number } | null {
    let best: { idx: number; d: number } | null = null;
    const threshold = 10;
    for (let i = 0; i < shadowQuad.length; i++) {
      const d = Math.hypot(p.x - shadowQuad[i].x, p.y - shadowQuad[i].y);
      if (d <= threshold && (!best || d < best.d)) {
        best = { idx: i, d };
      }
    }
    return best;
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const p = aggPos(e);
    if (inControlBounds(p)) return;

    const near = nearestVertex(p);
    if (near) {
      drag = {
        kind: 'vertex',
        idx: near.idx,
        dx: p.x - shadowQuad[near.idx].x,
        dy: p.y - shadowQuad[near.idx].y,
      };
      canvas.setPointerCapture(e.pointerId);
      return;
    }

    if (pointInPoly(p, shadowQuad)) {
      drag = { kind: 'all', lastX: p.x, lastY: p.y };
      canvas.setPointerCapture(e.pointerId);
    }
  }

  function onPointerMove(e: PointerEvent) {
    if (!drag) return;
    const p = aggPos(e);
    if (drag.kind === 'vertex') {
      shadowQuad[drag.idx].x = p.x - drag.dx;
      shadowQuad[drag.idx].y = p.y - drag.dy;
    } else {
      const dx = p.x - drag.lastX;
      const dy = p.y - drag.lastY;
      for (const v of shadowQuad) {
        v.x += dx;
        v.y += dy;
      }
      drag.lastX = p.x;
      drag.lastY = p.y;
    }
    draw();
  }

  function onPointerUp() {
    drag = null;
  }

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);

  draw();
  return () => {
    cleanupCC();
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
  };
}
