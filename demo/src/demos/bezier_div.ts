import { createDemoLayout, addSlider, addCheckbox, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupVertexDrag, Vertex } from '../mouse-helpers.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Bezier Div',
    'Cubic Bezier curve with draggable control points — matching C++ bezier_div.cpp.',
  );

  const W = 655, H = 520;

  const vertices: Vertex[] = [
    { x: 170, y: 424 },
    { x: 13, y: 87 },
    { x: 488, y: 423 },
    { x: 26, y: 333 },
  ];

  let angleTolerance = 15.0;
  let approximationScale = 1.0;
  let cuspLimit = 0.0;
  let strokeWidth = 50.0;
  let showPoints = true;
  let showOutline = true;
  let curveType = 1; // 0=Incremental, 1=Subdiv
  let caseType = 0;
  let innerJoin = 3; // 0=Bevel,1=Miter,2=Jag,3=Round
  let lineJoin = 1;  // 0=Miter,1=MiterRevert,2=Round,3=Bevel,4=MiterRound
  let lineCap = 0;   // 0=Butt,1=Square,2=Round

  function applyCasePreset(index: number) {
    const setCurve = (x1: number, y1: number, x2: number, y2: number, x3: number, y3: number, x4: number, y4: number) => {
      vertices[0].x = x1; vertices[0].y = y1;
      vertices[1].x = x2; vertices[1].y = y2;
      vertices[2].x = x3; vertices[2].y = y3;
      vertices[3].x = x4; vertices[3].y = y4;
    };

    switch (index) {
      case 0: { // Random
        const rw = W - 120;
        const rh = H - 80;
        const r = (n: number) => Math.floor(Math.random() * n);
        setCurve(r(rw), r(rh) + 80, r(rw), r(rh) + 80, r(rw), r(rh) + 80, r(rw), r(rh) + 80);
        break;
      }
      case 1: setCurve(150, 150, 350, 150, 150, 150, 350, 150); break; // 13---24
      case 2: setCurve(50, 142, 483, 251, 496, 62, 26, 333); break; // Smooth Cusp 1
      case 3: setCurve(50, 142, 484, 251, 496, 62, 26, 333); break; // Smooth Cusp 2
      case 4: setCurve(100, 100, 300, 200, 200, 200, 200, 100); break; // Real Cusp 1
      case 5: setCurve(475, 157, 200, 100, 453, 100, 222, 157); break; // Real Cusp 2
      case 6: // Fancy Stroke
        setCurve(129, 233, 32, 283, 258, 285, 159, 232);
        strokeWidth = 100.0;
        break;
      case 7: setCurve(100, 100, 300, 200, 264, 286, 264, 284); break; // Jaw
      case 8: setCurve(100, 100, 413, 304, 264, 286, 264, 284); break; // Ugly Jaw
      default: break;
    }
  }

  function draw() {
    renderToCanvas({
      demoName: 'bezier_div',
      canvas, width: W, height: H,
      params: [
        vertices[0].x, vertices[0].y,
        vertices[1].x, vertices[1].y,
        vertices[2].x, vertices[2].y,
        vertices[3].x, vertices[3].y,
        strokeWidth,
        showPoints ? 1 : 0,
        showOutline ? 1 : 0,
        angleTolerance,
        approximationScale,
        cuspLimit,
        curveType,
        caseType,
        innerJoin,
        lineJoin,
        lineCap,
      ],
      timeDisplay: timeEl,
    });
  }

  const cleanupDrag = setupVertexDrag({
    canvas,
    vertices,
    threshold: 10,
    onDrag: draw,
  });

  const slAngle = addSlider(sidebar, 'Angle Tolerance (deg)', 0, 90, angleTolerance, 1, v => { angleTolerance = v; draw(); });
  const slApprox = addSlider(sidebar, 'Approximation Scale', 0.1, 5.0, approximationScale, 0.01, v => { approximationScale = v; draw(); });
  const slCusp = addSlider(sidebar, 'Cusp Limit (deg)', 0, 90, cuspLimit, 1, v => { cuspLimit = v; draw(); });
  const slWidth = addSlider(sidebar, 'Width', -50, 100, 50.0, 1, v => { strokeWidth = v; draw(); });
  const cbPts = addCheckbox(sidebar, 'Show Points', true, v => { showPoints = v; draw(); });
  const cbOutline = addCheckbox(sidebar, 'Show Stroke Outline', true, v => { showOutline = v; draw(); });
  const curveTypeEls = addRadioGroup(sidebar, 'Curve Type', ['Incremental', 'Subdiv'], curveType,
    v => { curveType = v; draw(); });
  const caseTypeEls = addRadioGroup(sidebar, 'Case', [
    'Random', '13---24', 'Smooth Cusp 1', 'Smooth Cusp 2', 'Real Cusp 1',
    'Real Cusp 2', 'Fancy Stroke', 'Jaw', 'Ugly Jaw',
  ], caseType, v => {
    caseType = v;
    applyCasePreset(v);
    if (v === 6) {
      slWidth.value = String(strokeWidth);
      slWidth.dispatchEvent(new Event('input'));
      return;
    }
    draw();
  });
  const innerJoinEls = addRadioGroup(sidebar, 'Inner Join', ['Inner Bevel', 'Inner Miter', 'Inner Jag', 'Inner Round'], innerJoin,
    v => { innerJoin = v; draw(); });
  const lineJoinEls = addRadioGroup(sidebar, 'Line Join', ['Miter Join', 'Miter Revert', 'Round Join', 'Bevel Join', 'Miter Round'], lineJoin,
    v => { lineJoin = v; draw(); });
  const lineCapEls = addRadioGroup(sidebar, 'Line Cap', ['Butt Cap', 'Square Cap', 'Round Cap'], lineCap,
    v => { lineCap = v; draw(); });

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 240, y2: 12, min: 0, max: 90, sidebarEl: slAngle, onChange: v => { angleTolerance = v; draw(); } },
    { type: 'slider', x1: 5, y1: 22, x2: 240, y2: 29, min: 0.1, max: 5, sidebarEl: slApprox, onChange: v => { approximationScale = v; draw(); } },
    { type: 'slider', x1: 5, y1: 39, x2: 240, y2: 46, min: 0, max: 90, sidebarEl: slCusp, onChange: v => { cuspLimit = v; draw(); } },
    { type: 'slider', x1: 245, y1: 5, x2: 495, y2: 12, min: -50, max: 100, sidebarEl: slWidth, onChange: v => { strokeWidth = v; draw(); } },
    { type: 'checkbox', x1: 250, y1: 20, x2: 400, y2: 35, sidebarEl: cbPts, onChange: v => { showPoints = v; draw(); } },
    { type: 'checkbox', x1: 250, y1: 35, x2: 450, y2: 50, sidebarEl: cbOutline, onChange: v => { showOutline = v; draw(); } },
    { type: 'radio', x1: 535, y1: 5, x2: 650, y2: 55, numItems: 2, sidebarEls: curveTypeEls, onChange: v => { curveType = v; draw(); } },
    {
      type: 'radio', x1: 535, y1: 60, x2: 650, y2: 195, numItems: 9, sidebarEls: caseTypeEls, onChange: v => {
        caseType = v;
        applyCasePreset(v);
        if (v === 6) {
          slWidth.value = String(strokeWidth);
          slWidth.dispatchEvent(new Event('input'));
          return;
        }
        draw();
      },
    },
    { type: 'radio', x1: 535, y1: 200, x2: 650, y2: 290, numItems: 4, sidebarEls: innerJoinEls, onChange: v => { innerJoin = v; draw(); } },
    { type: 'radio', x1: 535, y1: 295, x2: 650, y2: 385, numItems: 5, sidebarEls: lineJoinEls, onChange: v => { lineJoin = v; draw(); } },
    { type: 'radio', x1: 535, y1: 395, x2: 650, y2: 455, numItems: 3, sidebarEls: lineCapEls, onChange: v => { lineCap = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: 'bottom-left' });

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the 4 control points. Canvas controls match C++ bezier_div.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupDrag(); cleanupCC(); };
}
