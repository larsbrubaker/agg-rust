import { createDemoLayout, addSlider, addCheckbox, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';
import { renderDemo } from '../wasm.ts';

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
  let rotating = false;
  let perfTesting = false;
  let animId = 0;

  function draw() {
    renderToCanvas({
      demoName: 'rasterizers2',
      canvas, width: W, height: H,
      params: [step, lineWidth, accurateJoins, startAngle, scalePattern, rotating ? 1 : 0, perfTesting ? 1 : 0],
      timeDisplay: timeEl,
    });
  }

  function startStop(v: boolean) {
    if (perfTesting && v) {
      cbRotate.checked = false;
      return;
    }
    if (v !== rotating) {
      rotating = v;
      draw();
    }
  }

  function tick() {
    if (rotating) {
      startAngle += step;
      if (startAngle > 360) startAngle -= 360;
      draw();
    }
    animId = requestAnimationFrame(tick);
  }

  async function runPerformanceTest() {
    if (perfTesting) return;
    perfTesting = true;
    const wasRotating = rotating;
    try {
      if (wasRotating) startStop(false);

      cbRotate.checked = false;
      cbTest.checked = true;
      draw();
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

      const iterations = 200;
      let benchAngle = startAngle;
      const t0 = performance.now();
      for (let i = 0; i < iterations; i++) {
        benchAngle += step;
        if (benchAngle > 360) benchAngle -= 360;
        renderDemo('rasterizers2', W, H, [step, lineWidth, accurateJoins, benchAngle, scalePattern, 0, 1]);
      }
      const elapsed = performance.now() - t0;

      startAngle = benchAngle;
      draw();
      window.alert(
        `Rasterizers2 benchmark (${iterations} frames)\n` +
        `Total: ${elapsed.toFixed(2)} ms\n` +
        `Average: ${(elapsed / iterations).toFixed(3)} ms/frame`,
      );
    } finally {
      perfTesting = false;
      cbTest.checked = false;
      draw();
      if (wasRotating) {
        cbRotate.checked = true;
        startStop(true);
      }
    }
  }

  // Sidebar controls — match C++ control set
  const slStep = addSlider(sidebar, 'Step', 0, 2, 0.1, 0.01, v => { step = v; draw(); });
  const slWidth = addSlider(sidebar, 'Width', 0, 14, 3, 0.01, v => { lineWidth = v; draw(); });
  const cbTest = addCheckbox(sidebar, 'Test Performance', false, v => {
    if (v) {
      void runPerformanceTest();
    }
  });
  const cbRotate = addCheckbox(sidebar, 'Rotate', false, v => startStop(v));
  const cbAccurate = addCheckbox(sidebar, 'Accurate Joins', false, v => { accurateJoins = v ? 1 : 0; draw(); });
  const cbScale = addCheckbox(sidebar, 'Scale Pattern', true, v => { scalePattern = v ? 1 : 0; draw(); });

  // Canvas controls — hit areas matching AGG-rendered controls
  // C++ layout: step slider (10,14)-(150,22), width slider (160,14)-(390,22)
  // Checkboxes at y=30: Test(10), Rotate(140), Accurate Joins(210), Scale Pattern(320)
  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 10, y1: 14, x2: 150, y2: 22, min: 0, max: 2, sidebarEl: slStep, onChange: v => { step = v; draw(); } },
    { type: 'slider', x1: 160, y1: 14, x2: 390, y2: 22, min: 0, max: 14, sidebarEl: slWidth, onChange: v => { lineWidth = v; draw(); } },
    { type: 'checkbox', x1: 10, y1: 30, x2: 130, y2: 44, sidebarEl: cbTest, onChange: v => { if (v) void runPerformanceTest(); } },
    { type: 'checkbox', x1: 140, y1: 30, x2: 200, y2: 44, sidebarEl: cbRotate, onChange: v => startStop(v) },
    { type: 'checkbox', x1: 210, y1: 30, x2: 310, y2: 44, sidebarEl: cbAccurate, onChange: v => { accurateJoins = v ? 1 : 0; draw(); } },
    { type: 'checkbox', x1: 320, y1: 30, x2: 420, y2: 44, sidebarEl: cbScale, onChange: v => { scalePattern = v ? 1 : 0; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw);

  draw();
  animId = requestAnimationFrame(tick);
  return () => {
    cancelAnimationFrame(animId);
    cleanupCC();
  };
}
