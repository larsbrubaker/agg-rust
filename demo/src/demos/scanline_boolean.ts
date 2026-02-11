import {
  createDemoLayout,
  addCheckbox,
  addRadioGroup,
  addSlider,
  renderToCanvas,
} from '../render-canvas.ts';
import { CanvasControl, setupCanvasControls } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Scanline Boolean',
    'Two overlapping circle groups combined with boolean operations — matching C++ scanline_boolean.cpp.',
  );

  const W = 800;
  const H = 600;

  type Vertex = { x: number; y: number };
  type Quad = [Vertex, Vertex, Vertex, Vertex];

  const defaultQuad1 = (): Quad => ([
    { x: 50, y: 180 },
    { x: W / 2 - 25, y: 200 },
    { x: W / 2 - 25, y: H - 70 },
    { x: 50, y: H - 50 },
  ]);
  const defaultQuad2 = (): Quad => ([
    { x: W / 2 + 25, y: 180 },
    { x: W - 50, y: 200 },
    { x: W - 50, y: H - 70 },
    { x: W / 2 + 25, y: H - 50 },
  ]);

  let operation = 0; // Union
  let opacity1 = 1.0;
  let opacity2 = 1.0;
  let quad1 = defaultQuad1();
  let quad2 = defaultQuad2();

  function packedQuad(q: Quad): number[] {
    return [q[0].x, q[0].y, q[1].x, q[1].y, q[2].x, q[2].y, q[3].x, q[3].y];
  }

  function draw() {
    renderToCanvas({
      demoName: 'scanline_boolean',
      canvas, width: W, height: H,
      params: [
        operation,
        opacity1,
        opacity2,
        ...packedQuad(quad1),
        ...packedQuad(quad2),
      ],
      timeDisplay: timeEl,
    });
  }

  const opRadios = addRadioGroup(sidebar, 'Operation', [
    'Union',
    'Intersection',
    'Linear XOR',
    'Saddle XOR',
    'Abs Diff XOR',
    'A-B',
    'B-A',
  ], operation, (i) => {
    operation = i;
    draw();
  });

  const opacity1Slider = addSlider(sidebar, 'Opacity1', 0, 1, opacity1, 0.001, (v) => {
    opacity1 = v;
    draw();
  });
  const opacity2Slider = addSlider(sidebar, 'Opacity2', 0, 1, opacity2, 0.001, (v) => {
    opacity2 = v;
    draw();
  });

  const resetCheckbox = addCheckbox(sidebar, 'Reset', false, (checked) => {
    if (!checked) return;
    quad1 = defaultQuad1();
    quad2 = defaultQuad2();
    resetCheckbox.checked = false;
    draw();
  });

  const canvasControls: CanvasControl[] = [
    {
      type: 'slider',
      x1: 5, y1: 5, x2: 340, y2: 12,
      min: 0, max: 1,
      sidebarEl: opacity1Slider,
      onChange: (v) => { opacity1 = v; draw(); },
    },
    {
      type: 'slider',
      x1: 5, y1: 20, x2: 340, y2: 27,
      min: 0, max: 1,
      sidebarEl: opacity2Slider,
      onChange: (v) => { opacity2 = v; draw(); },
    },
    {
      type: 'checkbox',
      x1: 350, y1: 5, x2: 410, y2: 20,
      sidebarEl: resetCheckbox,
      onChange: () => {},
    },
    {
      type: 'radio',
      x1: 420, y1: 5, x2: 550, y2: 145,
      numItems: 7,
      sidebarEls: opRadios,
      onChange: (i) => { operation = i; draw(); },
    },
  ];
  const cleanupCanvasControls = setupCanvasControls(canvas, canvasControls, draw);

  function aggPos(e: PointerEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: H - (e.clientY - rect.top) * scaleY,
    };
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

  let drag:
    | { kind: 'vertex'; quad: 1 | 2; idx: number; dx: number; dy: number }
    | { kind: 'all'; quad: 1 | 2; lastX: number; lastY: number }
    | null = null;

  function nearestVertex(p: Vertex): { quad: 1 | 2; idx: number; d: number } | null {
    let best: { quad: 1 | 2; idx: number; d: number } | null = null;
    const threshold = 10;
    const candidates: Array<{ quad: 1 | 2; poly: Quad }> = [
      { quad: 1, poly: quad1 },
      { quad: 2, poly: quad2 },
    ];
    for (const c of candidates) {
      for (let i = 0; i < c.poly.length; i++) {
        const v = c.poly[i];
        const d = Math.hypot(p.x - v.x, p.y - v.y);
        if (d <= threshold && (!best || d < best.d)) {
          best = { quad: c.quad, idx: i, d };
        }
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
      const poly = near.quad === 1 ? quad1 : quad2;
      drag = {
        kind: 'vertex',
        quad: near.quad,
        idx: near.idx,
        dx: p.x - poly[near.idx].x,
        dy: p.y - poly[near.idx].y,
      };
      canvas.setPointerCapture(e.pointerId);
      return;
    }

    if (pointInPoly(p, quad1)) {
      drag = { kind: 'all', quad: 1, lastX: p.x, lastY: p.y };
      canvas.setPointerCapture(e.pointerId);
      return;
    }
    if (pointInPoly(p, quad2)) {
      drag = { kind: 'all', quad: 2, lastX: p.x, lastY: p.y };
      canvas.setPointerCapture(e.pointerId);
    }
  }

  function onPointerMove(e: PointerEvent) {
    if (!drag) return;
    const p = aggPos(e);

    if (drag.kind === 'vertex') {
      const poly = drag.quad === 1 ? quad1 : quad2;
      poly[drag.idx].x = p.x - drag.dx;
      poly[drag.idx].y = p.y - drag.dy;
    } else {
      const poly = drag.quad === 1 ? quad1 : quad2;
      const ddx = p.x - drag.lastX;
      const ddy = p.y - drag.lastY;
      for (const v of poly) {
        v.x += ddx;
        v.y += ddy;
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
    cleanupCanvasControls();
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
  };
}
