import { createDemoLayout, addSlider, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Distortions',
    'Animated wave/swirl distortions on image and gradient sources — matching C++ distortions.cpp.',
  );

  const W = 620, H = 360;
  let angle = 20.0;
  let scale = 1.0;
  let amplitude = 10.0;
  let period = 1.0;
  let distType = 0;
  let centerX = 170.0;
  let centerY = 200.0;
  let phase = 0.0;
  let draggingCenter = false;
  let animationId = 0;

  function draw() {
    renderToCanvas({
      demoName: 'distortions',
      canvas, width: W, height: H,
      params: [angle, scale, amplitude, period, distType, centerX, centerY, phase],
      timeDisplay: timeEl,
    });
  }

  const slAngle = addSlider(sidebar, 'Angle', -180, 180, 20, 1, v => { angle = v; draw(); });
  const slScale = addSlider(sidebar, 'Scale', 0.1, 5.0, 1.0, 0.01, v => { scale = v; draw(); });
  const slAmp = addSlider(sidebar, 'Amplitude', 0.1, 40.0, 10.0, 0.1, v => { amplitude = v; draw(); });
  const slPeriod = addSlider(sidebar, 'Period', 0.1, 2.0, 1.0, 0.01, v => { period = v; draw(); });

  const radioButtons = addRadioGroup(
    sidebar,
    'Distortion Type',
    ['Wave', 'Swirl', 'Wave-Swirl', 'Swirl-Wave'],
    distType,
    i => { distType = i; draw(); },
  );

  const canvasControls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 150, y2: 12, min: -180, max: 180, sidebarEl: slAngle, onChange: v => { angle = v; draw(); } },
    { type: 'slider', x1: 5, y1: 20, x2: 150, y2: 27, min: 0.1, max: 5.0, sidebarEl: slScale, onChange: v => { scale = v; draw(); } },
    { type: 'slider', x1: 175, y1: 5, x2: 320, y2: 12, min: 0.1, max: 2.0, sidebarEl: slPeriod, onChange: v => { period = v; draw(); } },
    { type: 'slider', x1: 175, y1: 20, x2: 320, y2: 27, min: 0.1, max: 40.0, sidebarEl: slAmp, onChange: v => { amplitude = v; draw(); } },
    { type: 'radio', x1: 480, y1: 5, x2: 600, y2: 90, numItems: 4, sidebarEls: radioButtons, onChange: i => { distType = i; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: 'bottom-left' });

  function eventToAgg(e: PointerEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    const x = (e.clientX - rect.left) * scaleX;
    const yTop = (e.clientY - rect.top) * scaleY;
    return { x, y: H - yTop };
  }

  function onPointerDown(e: PointerEvent): void {
    if (e.button !== 0) return;
    draggingCenter = true;
    canvas.setPointerCapture(e.pointerId);
    const p = eventToAgg(e);
    centerX = p.x;
    centerY = p.y;
    draw();
  }

  function onPointerMove(e: PointerEvent): void {
    if (!draggingCenter) return;
    const p = eventToAgg(e);
    centerX = p.x;
    centerY = p.y;
    draw();
  }

  function onPointerUp(e: PointerEvent): void {
    if (!draggingCenter) return;
    draggingCenter = false;
    if (canvas.hasPointerCapture(e.pointerId)) {
      canvas.releasePointerCapture(e.pointerId);
    }
  }

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);

  function tick(): void {
    phase += 15.0 * Math.PI / 180.0;
    if (phase > Math.PI * 200.0) phase -= Math.PI * 200.0;
    draw();
    animationId = requestAnimationFrame(tick);
  }

  draw();
  animationId = requestAnimationFrame(tick);

  return () => {
    cancelAnimationFrame(animationId);
    cleanupCC();
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
  };
}
