import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Simple Blur',
    'Lion with 3x3 box blur inside a draggable circle. Matching C++ simple_blur.cpp.',
  );

  const W = 512, H = 400;
  let blurCx = 100;
  let blurCy = 102;

  function draw() {
    renderToCanvas({
      demoName: 'simple_blur',
      canvas, width: W, height: H,
      params: [blurCx, blurCy],
      timeDisplay: timeEl,
    });
  }

  function aggPos(e: PointerEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    const scaleX = W / rect.width;
    const scaleY = H / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: H - (e.clientY - rect.top) * scaleY,
    };
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    canvas.setPointerCapture(e.pointerId);
    const p = aggPos(e);
    blurCx = p.x;
    blurCy = p.y;
    draw();
  }

  function onPointerMove(e: PointerEvent) {
    if ((e.buttons & 1) === 0) return;
    const p = aggPos(e);
    blurCx = p.x;
    blurCy = p.y;
    draw();
  }

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Left-drag: move the blur circle.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
  };
}
