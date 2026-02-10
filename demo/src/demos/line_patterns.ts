import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Line Patterns',
    'Drawing bezier curves with image patterns — port of C++ line_patterns.cpp. Each curve uses a different procedural pattern sampled along its length.',
  );

  const W = 500, H = 450;
  let scaleX = 1.0;
  let startX = 0.0;

  // Default bezier control points — exact match of C++ line_patterns.cpp
  // 9 curves × 4 control points × 2 coords = 72 values
  const points: number[] = [
     64,  19,  14, 126, 118, 266,  19, 265,
    112, 113, 178,  32, 200, 132, 125, 438,
    401,  24, 326, 149, 285,  11, 177,  77,
    188, 427, 129, 295,  19, 283,  25, 410,
    451, 346, 302, 218, 265, 441, 459, 400,
    454, 198,  14,  13, 220, 291, 483, 283,
    301, 398, 355, 231, 209, 211, 170, 353,
    484, 101, 222,  33, 486, 435, 487, 138,
    143, 147,  11,  45,  83, 427, 132, 197,
  ];

  function draw() {
    renderToCanvas({
      demoName: 'line_patterns',
      canvas, width: W, height: H,
      params: [scaleX, startX, ...points],
      timeDisplay: timeEl,
    });
  }

  // Sidebar slider widgets
  const slScale = addSlider(sidebar, 'Scale X', 0.2, 3, 1, 0.01, v => { scaleX = v; draw(); });
  const slStart = addSlider(sidebar, 'Start X', 0, 10, 0, 0.01, v => { startX = v; draw(); });

  // ------------------------------------------------------------------
  // Unified pointer handling: sliders AND control point dragging
  // ------------------------------------------------------------------

  // AGG slider bounding boxes (in screen coords, y-down)
  // Slider y1=5, y2=12 in AGG (y-flipped) = y1=H-12=438, y2=H-5=445 in screen
  const sliders = [
    { x1: 5, x2: 240, yTop: H - 12, yBot: H - 5, min: 0.2, max: 3, el: slScale, set: (v: number) => { scaleX = v; } },
    { x1: 250, x2: 495, yTop: H - 12, yBot: H - 5, min: 0, max: 10, el: slStart, set: (v: number) => { startX = v; } },
  ];

  const GRAB_RADIUS = 10;
  let dragMode: 'slider' | 'point' | null = null;
  let activeSliderIdx = -1;
  let dragPointIdx = -1;

  /** Canvas position in screen coords (origin top-left, y down). */
  function screenPos(e: PointerEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    return {
      x: (e.clientX - rect.left) * (W / rect.width),
      y: (e.clientY - rect.top) * (H / rect.height),
    };
  }

  function hitSlider(x: number, y: number): number {
    for (let i = 0; i < sliders.length; i++) {
      const s = sliders[i];
      const extra = (s.yBot - s.yTop) / 2;
      if (x >= s.x1 - extra && x <= s.x2 + extra &&
          y >= s.yTop - extra && y <= s.yBot + extra) {
        return i;
      }
    }
    return -1;
  }

  function sliderValue(idx: number, x: number): number {
    const s = sliders[idx];
    const xs1 = s.x1 + 1;
    const xs2 = s.x2 - 1;
    let t = (x - xs1) / (xs2 - xs1);
    t = Math.max(0, Math.min(1, t));
    return s.min + t * (s.max - s.min);
  }

  function findPoint(mx: number, my: number): number {
    let bestD2 = GRAB_RADIUS * GRAB_RADIUS;
    let bestIdx = -1;
    for (let i = 0; i < points.length; i += 2) {
      const dx = points[i] - mx;
      const dy = points[i + 1] - my;
      const d2 = dx * dx + dy * dy;
      if (d2 < bestD2) {
        bestD2 = d2;
        bestIdx = i;
      }
    }
    return bestIdx;
  }

  function onDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const p = screenPos(e);

    // Check sliders first (they're on top)
    const si = hitSlider(p.x, p.y);
    if (si >= 0) {
      dragMode = 'slider';
      activeSliderIdx = si;
      canvas.setPointerCapture(e.pointerId);
      const v = sliderValue(si, p.x);
      sliders[si].set(v);
      sliders[si].el.value = String(v);
      sliders[si].el.dispatchEvent(new Event('input'));
      e.preventDefault();
      return;
    }

    // Check control points
    const pi = findPoint(p.x, p.y);
    if (pi >= 0) {
      dragMode = 'point';
      dragPointIdx = pi;
      canvas.setPointerCapture(e.pointerId);
      e.preventDefault();
      return;
    }
  }

  function onMove(e: PointerEvent) {
    if (!dragMode) return;
    const p = screenPos(e);

    if (dragMode === 'slider') {
      const v = sliderValue(activeSliderIdx, p.x);
      sliders[activeSliderIdx].set(v);
      sliders[activeSliderIdx].el.value = String(v);
      sliders[activeSliderIdx].el.dispatchEvent(new Event('input'));
    } else if (dragMode === 'point') {
      points[dragPointIdx] = p.x;
      points[dragPointIdx + 1] = p.y;
      draw();
    }
    e.preventDefault();
  }

  function onUp() {
    dragMode = null;
    activeSliderIdx = -1;
    dragPointIdx = -1;
  }

  canvas.addEventListener('pointerdown', onDown);
  canvas.addEventListener('pointermove', onMove);
  canvas.addEventListener('pointerup', onUp);
  canvas.addEventListener('pointercancel', onUp);

  draw();

  return () => {
    canvas.removeEventListener('pointerdown', onDown);
    canvas.removeEventListener('pointermove', onMove);
    canvas.removeEventListener('pointerup', onUp);
    canvas.removeEventListener('pointercancel', onUp);
  };
}
