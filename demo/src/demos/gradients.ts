import { createDemoLayout, addRadioGroup, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Gradients',
    '6 gradient types with mouse interaction — matching C++ gradients.cpp.',
  );

  const W = 512, H = 400;

  let cx = 350;
  let cy = 280;
  let angle = 0;
  let scale = 1.0;
  let gradType = 0;
  const scaleX = 1.0;
  const scaleY = 1.0;
  let gammaKx1 = 1.0;
  let gammaKy1 = 1.0;
  let gammaKx2 = 1.0;
  let gammaKy2 = 1.0;
  type Pt = { x: number; y: number };
  const splineR: Pt[] = [];
  const splineG: Pt[] = [];
  const splineB: Pt[] = [];
  const splineA: Pt[] = [];
  for (let i = 0; i < 6; i++) {
    const x = i / 5;
    const y = 1 - x;
    splineR.push({ x, y });
    splineG.push({ x, y });
    splineB.push({ x, y });
    splineA.push({ x, y: 1 });
  }

  const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));
  const flattenSpline = (pts: Pt[]) => pts.flatMap(p => [p.x, p.y]);

  function draw() {
    renderToCanvas({
      demoName: 'gradients',
      canvas, width: W, height: H,
      params: [
        cx, cy, angle, scale, gradType, scaleX, scaleY, gammaKx1, gammaKy1, gammaKx2, gammaKy2,
        ...flattenSpline(splineR),
        ...flattenSpline(splineG),
        ...flattenSpline(splineB),
        ...flattenSpline(splineA),
      ],
      timeDisplay: timeEl,
    });
  }

  function canvasPos(e: PointerEvent) {
    const rect = canvas.getBoundingClientRect();
    const sx = canvas.width / rect.width;
    const sy = canvas.height / rect.height;
    const x = (e.clientX - rect.left) * sx;
    const yRaw = (e.clientY - rect.top) * sy;
    return {
      raw: { x, y: yRaw },
      agg: { x, y: canvas.height - yRaw },
    };
  }

  const splineBoxes = [
    { x1: 210, y1: 10, x2: 460, y2: 45, pts: splineR },
    { x1: 210, y1: 50, x2: 460, y2: 85, pts: splineG },
    { x1: 210, y1: 90, x2: 460, y2: 125, pts: splineB },
    { x1: 210, y1: 130, x2: 460, y2: 165, pts: splineA },
  ];

  type DragMode =
    | { kind: 'none' }
    | { kind: 'center'; lastX: number; lastY: number }
    | { kind: 'rotate'; pdx: number; pdy: number; prevScale: number; prevAngle: number }
    | { kind: 'gamma1'; pdx: number; pdy: number; useRaw: boolean }
    | { kind: 'gamma2'; pdx: number; pdy: number; useRaw: boolean }
    | { kind: 'spline'; boxIdx: number; idx: number; pdx: number; pdy: number; useRaw: boolean };
  let drag: DragMode = { kind: 'none' };

  function inControls(p: { x: number; y: number }) {
    if (p.x >= 10 && p.x <= 200 && p.y >= 10 && p.y <= 165) return true; // gamma
    if (p.x >= 10 && p.x <= 200 && p.y >= 180 && p.y <= 300) return true; // rbox
    for (const b of splineBoxes) {
      if (p.x >= b.x1 && p.x <= b.x2 && p.y >= b.y1 && p.y <= b.y2) return true;
    }
    return false;
  }

  function inSphere(p: { x: number; y: number }) {
    const dx = p.x - 350;
    const dy = p.y - 280;
    return dx * dx + dy * dy <= 110 * 110;
  }

  function gammaPoints() {
    const x1 = 10, y1 = 10, x2 = 200, y2 = 165, bw = 2;
    const textH = 8;
    const yc2 = y2 - textH * 2;
    const xs1 = x1 + bw, ys1 = y1 + bw, xs2 = x2 - bw, ys2 = yc2 - bw * 0.5;
    return {
      xs1, ys1, xs2, ys2,
      p1x: xs1 + (xs2 - xs1) * gammaKx1 * 0.25,
      p1y: ys1 + (ys2 - ys1) * gammaKy1 * 0.25,
      p2x: xs2 - (xs2 - xs1) * gammaKx2 * 0.25,
      p2y: ys2 - (ys2 - ys1) * gammaKy2 * 0.25,
    };
  }

  function splinePointToCanvas(boxIdx: number, idx: number) {
    const b = splineBoxes[boxIdx];
    const bw = 1;
    const xs1 = b.x1 + bw, ys1 = b.y1 + bw, xs2 = b.x2 - bw, ys2 = b.y2 - bw;
    return {
      x: xs1 + (xs2 - xs1) * b.pts[idx].x,
      y: ys1 + (ys2 - ys1) * b.pts[idx].y,
      xs1, ys1, xs2, ys2,
    };
  }

  function onPointerDown(e: PointerEvent) {
    const pos = canvasPos(e);
    const useRawForControls = !inControls(pos.agg) && inControls(pos.raw);
    const p = useRawForControls ? pos.raw : pos.agg;
    const btn = e.button;
    canvas.setPointerCapture(e.pointerId);

    const g = gammaPoints();
    const d1 = Math.hypot(p.x - g.p1x, p.y - g.p1y);
    if (d1 <= 8) {
      drag = { kind: 'gamma1', pdx: g.p1x - p.x, pdy: g.p1y - p.y, useRaw: useRawForControls };
      return;
    }
    const d2 = Math.hypot(p.x - g.p2x, p.y - g.p2y);
    if (d2 <= 8) {
      drag = { kind: 'gamma2', pdx: g.p2x - p.x, pdy: g.p2y - p.y, useRaw: useRawForControls };
      return;
    }

    for (let bi = 0; bi < splineBoxes.length; bi++) {
      for (let i = 0; i < 6; i++) {
        const sp = splinePointToCanvas(bi, i);
        if (Math.hypot(p.x - sp.x, p.y - sp.y) <= 7) {
          drag = { kind: 'spline', boxIdx: bi, idx: i, pdx: sp.x - p.x, pdy: sp.y - p.y, useRaw: useRawForControls };
          return;
        }
      }
    }

    if (p.x >= 10 && p.x <= 200 && p.y >= 180 && p.y <= 300) {
      const item = clamp(Math.floor((p.y - 182) / 18), 0, 5);
      gradType = item;
      draw();
      drag = { kind: 'none' };
      return;
    }

    if (btn === 0 && inSphere(p)) {
      // Delta-based move: no position snap on mouse-down.
      drag = { kind: 'center', lastX: p.x, lastY: p.y };
    } else if (btn === 2 && inSphere(p)) {
      drag = { kind: 'rotate', pdx: cx - p.x, pdy: cy - p.y, prevScale: scale, prevAngle: angle + Math.PI };
    }
  }

  function onPointerMove(e: PointerEvent) {
    if (drag.kind === 'none' || e.buttons === 0) return;
    const pos = canvasPos(e);
    const p = ('useRaw' in drag && drag.useRaw) ? pos.raw : pos.agg;
    if (drag.kind === 'center' && (e.buttons & 1)) {
      const dx = p.x - drag.lastX;
      const dy = p.y - drag.lastY;
      cx += dx;
      cy += dy;
      drag.lastX = p.x;
      drag.lastY = p.y;
      draw();
      return;
    }
    if (drag.kind === 'rotate' && (e.buttons & 2)) {
      const dx = p.x - cx;
      const dy = p.y - cy;
      const dist = Math.hypot(dx, dy);
      const prevDist = Math.hypot(drag.pdx, drag.pdy);
      if (prevDist > 1) scale = drag.prevScale * dist / prevDist;
      angle = drag.prevAngle + Math.atan2(dy, dx) - Math.atan2(drag.pdy, drag.pdx);
      draw();
      return;
    }
    if (drag.kind === 'gamma1' || drag.kind === 'gamma2') {
      const gp = gammaPoints();
      const x = p.x + drag.pdx;
      const y = p.y + drag.pdy;
      if (drag.kind === 'gamma1') {
        gammaKx1 = clamp((x - gp.xs1) * 4 / (gp.xs2 - gp.xs1), 0.001, 1.999);
        gammaKy1 = clamp((y - gp.ys1) * 4 / (gp.ys2 - gp.ys1), 0.001, 1.999);
      } else {
        gammaKx2 = clamp((gp.xs2 - x) * 4 / (gp.xs2 - gp.xs1), 0.001, 1.999);
        gammaKy2 = clamp((gp.ys2 - y) * 4 / (gp.ys2 - gp.ys1), 0.001, 1.999);
      }
      draw();
      return;
    }
    if (drag.kind === 'spline') {
      const { boxIdx, idx } = drag;
      const sp = splinePointToCanvas(boxIdx, idx);
      const pts = splineBoxes[boxIdx].pts;
      let nx = clamp((p.x + drag.pdx - sp.xs1) / (sp.xs2 - sp.xs1), 0, 1);
      const ny = clamp((p.y + drag.pdy - sp.ys1) / (sp.ys2 - sp.ys1), 0, 1);
      if (idx === 0) nx = 0;
      else if (idx === 5) nx = 1;
      else nx = clamp(nx, pts[idx - 1].x + 0.001, pts[idx + 1].x - 0.001);
      pts[idx].x = nx;
      pts[idx].y = ny;
      draw();
    }
  }

  function onPointerUp() {
    drag = { kind: 'none' };
  }

  function onContextMenu(e: Event) {
    e.preventDefault();
  }

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);
  canvas.addEventListener('contextmenu', onContextMenu);

  addRadioGroup(sidebar, 'Gradient Type',
    ['Radial', 'Diamond', 'Linear', 'XY', 'Sqrt XY', 'Conic'],
    0,
    v => { gradType = v; draw(); },
  );

  addSlider(sidebar, 'Gamma kx1', 0.001, 1.999, gammaKx1, 0.001, v => { gammaKx1 = v; draw(); });
  addSlider(sidebar, 'Gamma ky1', 0.001, 1.999, gammaKy1, 0.001, v => { gammaKy1 = v; draw(); });
  addSlider(sidebar, 'Gamma kx2', 0.001, 1.999, gammaKx2, 0.001, v => { gammaKx2 = v; draw(); });
  addSlider(sidebar, 'Gamma ky2', 0.001, 1.999, gammaKy2, 0.001, v => { gammaKy2 = v; draw(); });

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Canvas controls now work: drag gamma/spline points, click gradient type, left-drag center, right-drag rotate/scale.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
    canvas.removeEventListener('contextmenu', onContextMenu);
  };
}
