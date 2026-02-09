import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Lion Lens',
    'Magnifying lens effect on the lion using trans_warp_magnifier — matching C++ lion_lens.cpp.',
  );

  const W = 512, H = 400;
  let magn = 3.0;
  let radius = 70.0;
  let lensX = W / 2;
  let lensY = H / 2;
  let angle = 0;

  function draw() {
    renderToCanvas({
      demoName: 'lion_lens',
      canvas, width: W, height: H,
      params: [magn, radius, lensX, lensY, angle],
      timeDisplay: timeEl,
    });
  }

  const slMagn = addSlider(sidebar, 'Magnification', 0.01, 4.0, 3.0, 0.01, v => { magn = v; draw(); });
  const slRadius = addSlider(sidebar, 'Radius', 0, 100, 70, 1, v => { radius = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 150, y2: 12, min: 0.01, max: 4.0, sidebarEl: slMagn, onChange: v => { magn = v; draw(); } },
    { type: 'slider', x1: 5, y1: 20, x2: 150, y2: 27, min: 0, max: 100, sidebarEl: slRadius, onChange: v => { radius = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  // Mouse move to position lens
  const handleMouseMove = (e: MouseEvent) => {
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    lensX = (e.clientX - rect.left) * scaleX;
    lensY = (e.clientY - rect.top) * scaleY;
    draw();
  };
  canvas.addEventListener('mousemove', handleMouseMove);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Move mouse over the canvas to position the lens.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupCC(); canvas.removeEventListener('mousemove', handleMouseMove); };
}
