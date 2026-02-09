import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Distortions',
    'Wave and swirl distortion effects on a procedural image — matching C++ distortions.cpp.',
  );

  const W = 500, H = 350;
  let angle = 20.0;
  let scale = 1.0;
  let amplitude = 10.0;
  let period = 1.0;
  let distType = 0;

  function draw() {
    renderToCanvas({
      demoName: 'distortions',
      canvas, width: W, height: H,
      params: [angle, scale, amplitude, period, distType],
      timeDisplay: timeEl,
    });
  }

  const slAngle = addSlider(sidebar, 'Angle', -180, 180, 20, 1, v => { angle = v; draw(); });
  const slScale = addSlider(sidebar, 'Scale', 0.1, 5.0, 1.0, 0.01, v => { scale = v; draw(); });
  const slAmp = addSlider(sidebar, 'Amplitude', 0.1, 40.0, 10.0, 0.1, v => { amplitude = v; draw(); });
  const slPeriod = addSlider(sidebar, 'Period', 0.1, 2.0, 1.0, 0.01, v => { period = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 495, y2: 12, min: -180, max: 180, sidebarEl: slAngle, onChange: v => { angle = v; draw(); } },
    { type: 'slider', x1: 5, y1: 20, x2: 495, y2: 27, min: 0.1, max: 5.0, sidebarEl: slScale, onChange: v => { scale = v; draw(); } },
    { type: 'slider', x1: 5, y1: 35, x2: 495, y2: 42, min: 0.1, max: 40.0, sidebarEl: slAmp, onChange: v => { amplitude = v; draw(); } },
    { type: 'slider', x1: 5, y1: 50, x2: 495, y2: 57, min: 0.1, max: 2.0, sidebarEl: slPeriod, onChange: v => { period = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  // Radio buttons for distortion type
  const radioDiv = document.createElement('div');
  radioDiv.className = 'control-group';
  const radioLabel = document.createElement('label');
  radioLabel.className = 'control-label';
  radioLabel.textContent = 'Distortion Type';
  radioDiv.appendChild(radioLabel);
  const names = ['Wave', 'Swirl', 'Wave+Swirl', 'Swirl+Wave'];
  names.forEach((name, i) => {
    const row = document.createElement('label');
    row.style.display = 'block';
    row.style.cursor = 'pointer';
    row.style.marginBottom = '2px';
    const rb = document.createElement('input');
    rb.type = 'radio';
    rb.name = 'distortion_type';
    rb.value = String(i);
    rb.checked = i === distType;
    rb.addEventListener('change', () => { distType = i; draw(); });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(' ' + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);

  draw();
  return cleanupCC;
}
