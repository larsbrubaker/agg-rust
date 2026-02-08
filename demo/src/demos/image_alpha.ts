import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Image Alpha',
    'Image with brightness-to-alpha mapping over a random ellipse background — matching C++ image_alpha.cpp.',
  );

  const W = 512, H = 400;

  // 6 alpha curve control values
  const alphaValues = [1.0, 1.0, 1.0, 0.5, 0.5, 1.0];

  function draw() {
    renderToCanvas({
      demoName: 'image_alpha',
      canvas, width: W, height: H,
      params: [...alphaValues],
      timeDisplay: timeEl,
    });
  }

  const controls: CanvasControl[] = [];
  for (let i = 0; i < 6; i++) {
    controls.push({
      type: 'slider',
      label: `Alpha ${i}`,
      min: 0, max: 1, step: 0.01,
      initial: alphaValues[i],
      onChange(v) { alphaValues[i] = v; draw(); },
    });
  }
  const cleanupCC = setupCanvasControls(canvas, controls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Adjust the 6 alpha curve values to control brightness-to-alpha mapping.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupCC(); };
}
