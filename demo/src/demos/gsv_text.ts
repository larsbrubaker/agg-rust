import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'GSV Text',
    'Built-in vector text engine — no font file dependencies.',
  );

  const W = 600, H = 500;
  let textSize = 24;
  let strokeWidth = 1.0;

  function draw() {
    renderToCanvas({
      demoName: 'gsv_text',
      canvas, width: W, height: H,
      params: [textSize, strokeWidth, 20, 40],
      timeDisplay: timeEl,
    });
  }

  addSlider(sidebar, 'Text Size', 8, 64, 24, 2, v => { textSize = v; draw(); });
  addSlider(sidebar, 'Stroke Width', 0.3, 4, 1, 0.1, v => { strokeWidth = v; draw(); });

  draw();
}
