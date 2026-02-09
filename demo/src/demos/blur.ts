import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Blur',
    'Stack blur and recursive blur on colored shapes — matching C++ blur.cpp.',
  );

  const W = 440, H = 330;
  let radius = 15.0;
  let method = 0;

  function draw() {
    renderToCanvas({
      demoName: 'blur',
      canvas, width: W, height: H,
      params: [radius, method],
      timeDisplay: timeEl,
    });
  }

  const slRadius = addSlider(sidebar, 'Blur Radius', 0, 40, 15, 0.5, v => { radius = v; draw(); });

  // Method selector
  const methodDiv = document.createElement('div');
  methodDiv.className = 'control-group';
  const methodLabel = document.createElement('label');
  methodLabel.textContent = 'Method';
  methodDiv.appendChild(methodLabel);
  const methods = ['Stack Blur', 'Recursive Blur', 'Channels'];
  methods.forEach((name, i) => {
    const btn = document.createElement('button');
    btn.textContent = name;
    btn.className = i === method ? 'method-btn active' : 'method-btn';
    btn.style.cssText = 'margin: 2px; padding: 4px 8px; cursor: pointer; font-size: 12px;';
    if (i === method) btn.style.fontWeight = 'bold';
    btn.addEventListener('click', () => {
      method = i;
      methodDiv.querySelectorAll('.method-btn').forEach(b => {
        (b as HTMLElement).style.fontWeight = 'normal';
        b.className = 'method-btn';
      });
      btn.style.fontWeight = 'bold';
      btn.className = 'method-btn active';
      draw();
    });
    methodDiv.appendChild(btn);
  });
  sidebar.appendChild(methodDiv);

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 435, y2: 12, min: 0, max: 40, sidebarEl: slRadius, onChange: v => { radius = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  return cleanupCC;
}
