import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Line Patterns',
    'Anti-aliased outline spirals with configurable width and join modes — adapted from C++ line_patterns.cpp.',
  );

  const W = 500, H = 450;
  let lineWidth = 3.0;
  let accurateJoins = 0;
  let startAngle = 0;

  function draw() {
    renderToCanvas({
      demoName: 'line_patterns',
      canvas, width: W, height: H,
      params: [lineWidth, accurateJoins, startAngle],
      timeDisplay: timeEl,
    });
  }

  const slWidth = addSlider(sidebar, 'Width', 0.5, 10, 3, 0.01, v => { lineWidth = v; draw(); });
  addSlider(sidebar, 'Start Angle', 0, 360, 0, 1, v => { startAngle = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 10, y1: 14, x2: 490, y2: 22, min: 0.5, max: 10, sidebarEl: slWidth, onChange: v => { lineWidth = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  // Checkbox for accurate joins
  const cbDiv = document.createElement('div');
  cbDiv.className = 'control-group';
  const cb = document.createElement('input');
  cb.type = 'checkbox';
  cb.id = 'lp_accurate';
  cb.checked = false;
  cb.addEventListener('change', () => { accurateJoins = cb.checked ? 1 : 0; draw(); });
  const cbLabel = document.createElement('label');
  cbLabel.htmlFor = cb.id;
  cbLabel.textContent = ' Accurate Joins';
  cbDiv.appendChild(cb);
  cbDiv.appendChild(cbLabel);
  sidebar.appendChild(cbDiv);

  draw();
  return cleanupCC;
}
