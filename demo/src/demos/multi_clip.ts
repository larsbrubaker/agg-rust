import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupRotateScale } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Multi Clip',
    'Lion rendered through N×N clip regions with random shapes — matching C++ multi_clip.cpp.',
  );

  const W = 512, H = 400;
  let n = 4.0;
  let angle = 0;
  let scale = 1.0;

  function draw() {
    renderToCanvas({
      demoName: 'multi_clip',
      canvas, width: W, height: H,
      params: [n, angle, scale],
      timeDisplay: timeEl,
    });
  }

  const cleanupRS = setupRotateScale({
    canvas,
    onLeftDrag: (a, s) => { angle = a; scale = s; draw(); },
  });

  const slN = addSlider(sidebar, 'N', 2, 10, 4, 1, v => { n = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 150, y2: 12, min: 2, max: 10, sidebarEl: slN, onChange: v => { n = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Left-drag: rotate & scale.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupRS(); cleanupCC(); };
}
