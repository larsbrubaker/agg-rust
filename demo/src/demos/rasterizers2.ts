import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Rasterizers 2',
    'Comparison of different rasterization techniques: aliased, AA outline, scanline, and image pattern — matching C++ rasterizers2.cpp.',
  );

  const W = 500, H = 450;
  let step = 0.1;
  let lineWidth = 3.0;
  let accurateJoins = 0;
  let startAngle = 0;
  let scalePattern = 1;

  function draw() {
    renderToCanvas({
      demoName: 'rasterizers2',
      canvas, width: W, height: H,
      params: [step, lineWidth, accurateJoins, startAngle, scalePattern],
      timeDisplay: timeEl,
    });
  }

  const slWidth = addSlider(sidebar, 'Width', 0, 14, 3, 0.01, v => { lineWidth = v; draw(); });
  addSlider(sidebar, 'Start Angle', 0, 360, 0, 1, v => { startAngle = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 160, y1: 14, x2: 490, y2: 22, min: 0, max: 14, sidebarEl: slWidth, onChange: v => { lineWidth = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  // Helper to add a checkbox control
  function addCheckbox(id: string, label: string, checked: boolean, onChange: (v: boolean) => void) {
    const div = document.createElement('div');
    div.className = 'control-group';
    const inp = document.createElement('input');
    inp.type = 'checkbox';
    inp.id = id;
    inp.checked = checked;
    inp.addEventListener('change', () => onChange(inp.checked));
    const lbl = document.createElement('label');
    lbl.htmlFor = id;
    lbl.textContent = ' ' + label;
    div.appendChild(inp);
    div.appendChild(lbl);
    sidebar.appendChild(div);
  }

  addCheckbox('rast2_accurate', 'Accurate Joins', false, v => { accurateJoins = v ? 1 : 0; draw(); });
  addCheckbox('rast2_scale_pattern', 'Scale Pattern', true, v => { scalePattern = v ? 1 : 0; draw(); });

  draw();
  return cleanupCC;
}
