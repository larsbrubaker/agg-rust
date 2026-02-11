import { addRadioGroup, createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Alpha Mask 3',
    'Alpha mask polygon clipping with AND/SUB operations — matching C++ alpha_mask3.cpp.',
  );

  const W = 640, H = 520;

  let scenario = 3;
  let operation = 0;
  let mouseX = W / 2;
  let mouseY = H / 2;
  const scenarioLabels = [
    'Two Simple Paths',
    'Closed Stroke',
    'Great Britain and Arrows',
    'Great Britain and Spiral',
    'Spiral and Glyph',
  ];
  const operationLabels = ['AND', 'SUB'];

  function draw() {
    renderToCanvas({
      demoName: 'alpha_mask3',
      canvas, width: W, height: H,
      params: [scenario, operation, mouseX, mouseY],
      timeDisplay: timeEl,
    });
  }

  // Mouse drag for position (AGG uses lower-left origin).
  const aggMousePos = (e: PointerEvent): { x: number; y: number } => {
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    const x = (e.clientX - rect.left) * scaleX;
    const yTop = (e.clientY - rect.top) * scaleY;
    return { x, y: H - yTop };
  };

  let dragging = false;
  const onPointerDown = (e: PointerEvent) => {
    if (e.button === 0) {
      const p = aggMousePos(e);
      mouseX = p.x;
      mouseY = p.y;
      draw();
      dragging = true;
      canvas.setPointerCapture(e.pointerId);
    }
  };
  const onPointerMove = (e: PointerEvent) => {
    if (!dragging || (e.buttons & 1) === 0) return;
    const p = aggMousePos(e);
    mouseX = p.x;
    mouseY = p.y;
    draw();
  };
  const onPointerUp = () => { dragging = false; };
  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);

  // C++ rbox renders first item at the bottom. Show sidebar in the same visual order.
  const scenarioLabelsUi = [...scenarioLabels].reverse();
  const scenarioInputsUi = addRadioGroup(
    sidebar,
    'Polygons',
    scenarioLabelsUi,
    scenarioLabels.length - 1 - scenario,
    (uiIndex) => {
      scenario = scenarioLabels.length - 1 - uiIndex;
      draw();
    },
  );
  const scenarioInputs: HTMLInputElement[] = new Array(scenarioLabels.length);
  for (let uiIndex = 0; uiIndex < scenarioLabels.length; uiIndex++) {
    const logicalIndex = scenarioLabels.length - 1 - uiIndex;
    scenarioInputs[logicalIndex] = scenarioInputsUi[uiIndex];
  }

  const operationLabelsUi = [...operationLabels].reverse();
  const operationInputsUi = addRadioGroup(
    sidebar,
    'Operation',
    operationLabelsUi,
    operationLabels.length - 1 - operation,
    (uiIndex) => {
      operation = operationLabels.length - 1 - uiIndex;
      draw();
    },
  );
  const operationInputs: HTMLInputElement[] = new Array(operationLabels.length);
  for (let uiIndex = 0; uiIndex < operationLabels.length; uiIndex++) {
    const logicalIndex = operationLabels.length - 1 - uiIndex;
    operationInputs[logicalIndex] = operationInputsUi[uiIndex];
  }

  const controls: CanvasControl[] = [
    {
      type: 'radio',
      x1: 5,
      y1: 5,
      x2: 210,
      y2: 110,
      numItems: 5,
      sidebarEls: scenarioInputs,
      onChange: (index: number) => {
        scenario = index;
        draw();
      },
    },
    {
      type: 'radio',
      x1: 555,
      y1: 5,
      x2: 635,
      y2: 55,
      numItems: 2,
      sidebarEls: operationInputs,
      onChange: (index: number) => {
        operation = index;
        draw();
      },
    },
  ];
  const cleanupCC = setupCanvasControls(canvas, controls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Left-click or drag on canvas to move shapes. Canvas and sidebar controls are synchronized.';
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
