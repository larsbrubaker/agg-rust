import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'AA Test',
    'Radial dashes, ellipses, gradient lines, and Gouraud triangles — matching C++ aa_test.cpp.',
  );

  const W = 480, H = 350;
  let gamma = 1.6;

  function draw() {
    renderToCanvas({
      demoName: 'aa_test',
      canvas, width: W, height: H,
      params: [gamma],
      timeDisplay: timeEl,
    });
  }

  const slGamma = addSlider(sidebar, 'Gamma', 0.1, 3.0, 1.6, 0.01, v => { gamma = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 340, y2: 12, min: 0.1, max: 3, sidebarEl: slGamma, onChange: v => { gamma = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Anti-aliasing quality test: radial lines, ellipses at varying sizes, gradient lines, Gouraud triangles.';
  sidebar.appendChild(hint);

  draw();
  return cleanupCC;
}
