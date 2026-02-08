import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Strokes',
    'Line caps, joins, and variable stroke width.',
  );

  let baseWidth = 3.0;
  const W = 800, H = 600;

  function draw() {
    renderToCanvas({
      demoName: 'strokes',
      canvas, width: W, height: H,
      params: [baseWidth],
      timeDisplay: timeEl,
    });
  }

  addSlider(sidebar, 'Base Width', 0.5, 10.0, 3.0, 0.5, v => { baseWidth = v; draw(); });

  draw();
}
