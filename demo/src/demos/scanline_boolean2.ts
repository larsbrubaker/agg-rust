import { createDemoLayout, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Scanline Boolean 2',
    'Boolean operations on complex shapes — matching C++ scanline_boolean2.cpp.',
  );

  const W = 655, H = 520;

  // State
  let polygonIdx = 3;    // Great Britain and Spiral
  let fillRuleIdx = 1;   // Non Zero
  let scanlineIdx = 1;   // scanline_u
  let operationIdx = 2;  // AND
  let mouseX = W / 2;
  let mouseY = H / 2;
  let isDragging = false;

  function draw() {
    renderToCanvas({
      demoName: 'scanline_boolean2',
      canvas, width: W, height: H,
      params: [polygonIdx, fillRuleIdx, scanlineIdx, operationIdx, mouseX, mouseY],
      timeDisplay: timeEl,
    });
  }

  // Sidebar controls matching C++ layout
  const polyRadios = addRadioGroup(sidebar, 'Polygons', [
    'Two Simple Paths',
    'Closed Stroke',
    'Great Britain and Arrows',
    'Great Britain and Spiral',
    'Spiral and Glyph',
  ], polygonIdx, (i) => { polygonIdx = i; draw(); });

  const fillRadios = addRadioGroup(sidebar, 'Fill Rule', [
    'Even-Odd',
    'Non Zero',
  ], fillRuleIdx, (i) => { fillRuleIdx = i; draw(); });

  const slRadios = addRadioGroup(sidebar, 'Scanline Type', [
    'scanline_p',
    'scanline_u',
    'scanline_bin',
  ], scanlineIdx, (i) => { scanlineIdx = i; draw(); });

  const opRadios = addRadioGroup(sidebar, 'Operation', [
    'None',
    'OR',
    'AND',
    'XOR Linear',
    'XOR Saddle',
    'A-B',
    'B-A',
  ], operationIdx, (i) => { operationIdx = i; draw(); });

  // Canvas controls for clicking on the rendered radio buttons
  const canvasControls: CanvasControl[] = [
    { type: 'radio', x1: 5, y1: 5, x2: 210, y2: 110,
      numItems: 5, sidebarEls: polyRadios,
      onChange: (i) => { polygonIdx = i; draw(); } },
    { type: 'radio', x1: 200, y1: 5, x2: 305, y2: 50,
      numItems: 2, sidebarEls: fillRadios,
      onChange: (i) => { fillRuleIdx = i; draw(); } },
    { type: 'radio', x1: 300, y1: 5, x2: 415, y2: 70,
      numItems: 3, sidebarEls: slRadios,
      onChange: (i) => { scanlineIdx = i; draw(); } },
    { type: 'radio', x1: 535, y1: 5, x2: 650, y2: 145,
      numItems: 7, sidebarEls: opRadios,
      onChange: (i) => { operationIdx = i; draw(); } },
  ];
  setupCanvasControls(canvas, canvasControls, draw);

  // Mouse drag support for moving shapes
  function aggPos(e: MouseEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: H - (e.clientY - rect.top) * scaleY,
    };
  }

  canvas.addEventListener('pointerdown', (e) => {
    if (e.button !== 0) return;
    const pos = aggPos(e);
    // Don't start drag if inside a control
    for (const c of canvasControls) {
      if (pos.x >= c.x1 && pos.x <= c.x2 && pos.y >= c.y1 && pos.y <= c.y2) {
        return;
      }
    }
    isDragging = true;
    mouseX = pos.x;
    mouseY = pos.y;
    canvas.setPointerCapture(e.pointerId);
    draw();
  });

  canvas.addEventListener('pointermove', (e) => {
    if (!isDragging) return;
    const pos = aggPos(e);
    mouseX = pos.x;
    mouseY = pos.y;
    draw();
  });

  canvas.addEventListener('pointerup', () => {
    isDragging = false;
  });

  draw();
}
