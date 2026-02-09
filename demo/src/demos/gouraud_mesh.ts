import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container, 'Gouraud Mesh',
    'Gouraud-shaded triangle mesh with compound rasterizer — matching C++ gouraud_mesh.cpp.',
  );
  const W = 512, H = 400;
  let cols = 8, rows = 8, seed = 0;

  function draw() {
    renderToCanvas({ demoName: 'gouraud_mesh', canvas, width: W, height: H,
      params: [cols, rows, seed], timeDisplay: timeEl });
  }

  addSlider(sidebar, 'Columns', 3, 20, 8, 1, v => { cols = v; draw(); });
  addSlider(sidebar, 'Rows', 3, 20, 8, 1, v => { rows = v; draw(); });
  addSlider(sidebar, 'Color Seed', 0, 100, 0, 1, v => { seed = v; draw(); });

  draw();
}
