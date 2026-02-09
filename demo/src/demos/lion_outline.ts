import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Lion Outline',
    'Lion rendered with anti-aliased outline rasterizer vs scanline rasterizer — matching C++ lion_outline.cpp.',
  );

  const W = 512, H = 512;
  let angle = 0;
  let scale = 1.0;
  let skewX = 0;
  let skewY = 0;
  let lineWidth = 1.0;
  let useScanline = 0;

  function draw() {
    renderToCanvas({
      demoName: 'lion_outline',
      canvas, width: W, height: H,
      params: [angle, scale, skewX, skewY, lineWidth, useScanline],
      timeDisplay: timeEl,
    });
  }

  const slWidth = addSlider(sidebar, 'Width', 0, 4, 1, 0.01, v => { lineWidth = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 150, y2: 12, min: 0, max: 4, sidebarEl: slWidth, onChange: v => { lineWidth = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  // Checkbox for scanline mode
  const cbDiv = document.createElement('div');
  cbDiv.className = 'control-group';
  const cb = document.createElement('input');
  cb.type = 'checkbox';
  cb.id = 'lion_outline_scanline';
  cb.checked = false;
  cb.addEventListener('change', () => { useScanline = cb.checked ? 1 : 0; draw(); });
  const cbLabel = document.createElement('label');
  cbLabel.htmlFor = cb.id;
  cbLabel.textContent = ' Use Scanline Rasterizer';
  cbDiv.appendChild(cb);
  cbDiv.appendChild(cbLabel);
  sidebar.appendChild(cbDiv);

  // Mouse drag for rotation/scale
  let dragging = false;
  canvas.addEventListener('pointerdown', (e) => {
    dragging = true;
    canvas.setPointerCapture(e.pointerId);
    updateTransform(e);
  });
  canvas.addEventListener('pointermove', (e) => {
    if (dragging) updateTransform(e);
  });
  canvas.addEventListener('pointerup', () => { dragging = false; });

  function updateTransform(e: MouseEvent) {
    const rect = canvas.getBoundingClientRect();
    const sx = W / rect.width;
    const sy = H / rect.height;
    const x = (e.clientX - rect.left) * sx - W / 2;
    const y = (e.clientY - rect.top) * sy - H / 2;
    angle = Math.atan2(y, x);
    scale = Math.sqrt(x * x + y * y) / 100;
    draw();
  }

  draw();
  return cleanupCC;
}
