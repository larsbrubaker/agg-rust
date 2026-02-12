import { addSlider, createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { Vertex } from '../mouse-helpers.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Alpha Gradient',
    'Gradient with alpha curve control over a random ellipse background — matching C++ alpha_gradient.cpp.',
  );

  const W = 512, H = 400;

  // Triangle vertices for gradient center/direction
  const vertices: Vertex[] = [
    { x: 257, y: 60 },
    { x: 369, y: 170 },
    { x: 143, y: 310 },
  ];

  // 6 alpha curve control values (0..1)
  const alphaValues = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];

  const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));

  function pointInTriangle(px: number, py: number, a: Vertex, b: Vertex, c: Vertex): boolean {
    const s = (a.x - c.x) * (py - c.y) - (a.y - c.y) * (px - c.x);
    const t = (b.x - a.x) * (py - a.y) - (b.y - a.y) * (px - a.x);
    const u = (c.x - b.x) * (py - b.y) - (c.y - b.y) * (px - b.x);
    const hasNeg = (s < 0) || (t < 0) || (u < 0);
    const hasPos = (s > 0) || (t > 0) || (u > 0);
    return !(hasNeg && hasPos);
  }

  function canvasPos(e: PointerEvent) {
    const rect = canvas.getBoundingClientRect();
    const sx = canvas.width / rect.width;
    const sy = canvas.height / rect.height;
    const x = (e.clientX - rect.left) * sx;
    const yTop = (e.clientY - rect.top) * sy;
    return {
      raw: { x, y: yTop },
      agg: { x, y: canvas.height - yTop },
    };
  }

  const spline = {
    x1: 2.0, y1: 2.0, x2: 200.0, y2: 30.0, border: 1.0,
  };
  const splineXs1 = spline.x1 + spline.border;
  const splineYs1 = spline.y1 + spline.border;
  const splineXs2 = spline.x2 - spline.border;
  const splineYs2 = spline.y2 - spline.border;

  function inSplineRect(p: { x: number; y: number }): boolean {
    return p.x >= spline.x1 && p.x <= spline.x2 && p.y >= spline.y1 && p.y <= spline.y2;
  }

  function splinePoint(i: number) {
    return {
      x: splineXs1 + (splineXs2 - splineXs1) * (i / 5),
      y: splineYs1 + (splineYs2 - splineYs1) * alphaValues[i],
    };
  }

  function draw() {
    renderToCanvas({
      demoName: 'alpha_gradient',
      canvas, width: W, height: H,
      params: [
        ...vertices.flatMap(v => [v.x, v.y]),
        ...alphaValues,
      ],
      timeDisplay: timeEl,
    });
  }

  // Sidebar controls stay in sync with canvas spline control.
  const alphaSliders: HTMLInputElement[] = [];
  for (let i = 0; i < 6; i++) {
    alphaSliders.push(
      addSlider(sidebar, `Alpha ${i}`, 0, 1, alphaValues[i], 0.01, (v) => {
        alphaValues[i] = v;
        draw();
      }),
    );
  }

  type DragState =
    | { kind: 'none' }
    | { kind: 'vertex'; idx: number; dx: number; dy: number }
    | { kind: 'all'; dx: number; dy: number }
    | { kind: 'alpha'; idx: number; useRaw: boolean; pdy: number };
  let drag: DragState = { kind: 'none' };

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const pos = canvasPos(e);

    // Spline control hit test first (matches C++ add_ctrl interaction priority).
    const useRawForSpline = !inSplineRect(pos.agg) && inSplineRect(pos.raw);
    const pSpline = useRawForSpline ? pos.raw : pos.agg;
    for (let i = 0; i < 6; i++) {
      const sp = splinePoint(i);
      if (Math.hypot(pSpline.x - sp.x, pSpline.y - sp.y) <= 8.0) {
        drag = { kind: 'alpha', idx: i, useRaw: useRawForSpline, pdy: sp.y - pSpline.y };
        canvas.setPointerCapture(e.pointerId);
        e.preventDefault();
        e.stopPropagation();
        return;
      }
    }

    const p = pos.agg;
    for (let i = 0; i < 3; i++) {
      const d = Math.hypot(p.x - vertices[i].x, p.y - vertices[i].y);
      if (d < 10.0) {
        drag = { kind: 'vertex', idx: i, dx: p.x - vertices[i].x, dy: p.y - vertices[i].y };
        canvas.setPointerCapture(e.pointerId);
        return;
      }
    }

    if (pointInTriangle(p.x, p.y, vertices[0], vertices[1], vertices[2])) {
      drag = { kind: 'all', dx: p.x - vertices[0].x, dy: p.y - vertices[0].y };
      canvas.setPointerCapture(e.pointerId);
    }
  }

  function onPointerMove(e: PointerEvent) {
    if (drag.kind === 'none' || e.buttons === 0) return;
    const pos = canvasPos(e);

    if (drag.kind === 'alpha') {
      const p = drag.useRaw ? pos.raw : pos.agg;
      const y = p.y + drag.pdy;
      const v = clamp((y - splineYs1) / (splineYs2 - splineYs1), 0, 1);
      const slider = alphaSliders[drag.idx];
      slider.value = String(v);
      slider.dispatchEvent(new Event('input'));
      e.preventDefault();
      e.stopPropagation();
      return;
    }

    const p = pos.agg;
    if (drag.kind === 'vertex') {
      vertices[drag.idx].x = p.x - drag.dx;
      vertices[drag.idx].y = p.y - drag.dy;
      draw();
      return;
    }

    const nx = p.x - drag.dx;
    const ny = p.y - drag.dy;
    const ddx = nx - vertices[0].x;
    const ddy = ny - vertices[0].y;
    for (const v of vertices) {
      v.x += ddx;
      v.y += ddy;
    }
    draw();
  }

  function onPointerUp() {
    drag = { kind: 'none' };
  }

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag vertices or inside the triangle to move the parallelogram; drag spline points on canvas or use the sidebar sliders.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
  };
}
