import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Alpha Mask 3',
    'Alpha mask polygon clipping with AND/SUB operations — matching C++ alpha_mask3.cpp.',
  );

  const W = 640, H = 520;

  let scenario = 0;
  let operation = 0;
  let mouseX = W / 2;
  let mouseY = H / 2;

  function draw() {
    renderToCanvas({
      demoName: 'alpha_mask3',
      canvas, width: W, height: H,
      params: [scenario, operation, mouseX, mouseY],
      timeDisplay: timeEl,
    });
  }

  // Mouse drag for position
  let dragging = false;
  const onPointerDown = (e: PointerEvent) => {
    if (e.button === 0) {
      dragging = true;
      canvas.setPointerCapture(e.pointerId);
    }
  };
  const onPointerMove = (e: PointerEvent) => {
    if (!dragging) return;
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    mouseX = (e.clientX - rect.left) * scaleX;
    mouseY = (e.clientY - rect.top) * scaleY;
    draw();
  };
  const onPointerUp = () => { dragging = false; };
  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);

  // Scenario radio buttons
  const scenarioDiv = document.createElement('div');
  scenarioDiv.className = 'control-group';
  const scenarioLabel = document.createElement('label');
  scenarioLabel.className = 'control-label';
  scenarioLabel.textContent = 'Scenario';
  scenarioDiv.appendChild(scenarioLabel);
  const scenarioNames = ['Two Triangles', 'Star', 'Spiral', 'Pentagons', 'Hexagons'];
  scenarioNames.forEach((name, i) => {
    const row = document.createElement('label');
    row.style.display = 'block';
    row.style.cursor = 'pointer';
    row.style.marginBottom = '2px';
    const rb = document.createElement('input');
    rb.type = 'radio';
    rb.name = 'am3_scenario';
    rb.value = String(i);
    rb.checked = i === scenario;
    rb.addEventListener('change', () => { scenario = i; draw(); });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(' ' + name));
    scenarioDiv.appendChild(row);
  });
  sidebar.appendChild(scenarioDiv);

  // Operation radio buttons
  const opDiv = document.createElement('div');
  opDiv.className = 'control-group';
  const opLabel = document.createElement('label');
  opLabel.className = 'control-label';
  opLabel.textContent = 'Operation';
  opDiv.appendChild(opLabel);
  const opNames = ['AND (intersect)', 'SUB (subtract)'];
  opNames.forEach((name, i) => {
    const row = document.createElement('label');
    row.style.display = 'block';
    row.style.cursor = 'pointer';
    row.style.marginBottom = '2px';
    const rb = document.createElement('input');
    rb.type = 'radio';
    rb.name = 'am3_operation';
    rb.value = String(i);
    rb.checked = i === operation;
    rb.addEventListener('change', () => { operation = i; draw(); });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(' ' + name));
    opDiv.appendChild(row);
  });
  sidebar.appendChild(opDiv);

  const controls: CanvasControl[] = [];
  const cleanupCC = setupCanvasControls(canvas, controls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag on canvas to move the second polygon. Choose scenario and operation.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
    cleanupCC();
  };
}
