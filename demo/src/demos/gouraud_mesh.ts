import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { gouraudMeshPickVertex } from '../wasm.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container, 'Gouraud Mesh',
    'Animated Gouraud-shaded triangle mesh with draggable vertices — matching C++ gouraud_mesh.cpp.',
  );
  const W = 400;
  const H = 400;
  const PICK_RADIUS = 8;
  const overrides = new Map<number, { x: number; y: number }>();
  let dragIdx = -1;
  let frame = 0;
  let raf = 0;

  function canvasPos(e: PointerEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: canvas.height - (e.clientY - rect.top) * scaleY,
    };
  }

  function buildParams(animStep: number): number[] {
    const params: number[] = [animStep, overrides.size];
    for (const [idx, p] of overrides) {
      params.push(idx, p.x, p.y);
    }
    return params;
  }

  function draw() {
    renderToCanvas({
      demoName: 'gouraud_mesh',
      canvas,
      width: W,
      height: H,
      params: buildParams(frame),
      timeDisplay: timeEl,
    });
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const pos = canvasPos(e);
    dragIdx = gouraudMeshPickVertex(W, H, buildParams(frame), pos.x, pos.y, PICK_RADIUS);
    if (dragIdx >= 0) {
      overrides.set(dragIdx, { x: pos.x, y: pos.y });
      canvas.setPointerCapture(e.pointerId);
      draw();
    }
  }

  function onPointerMove(e: PointerEvent) {
    if (dragIdx < 0) return;
    const pos = canvasPos(e);
    overrides.set(dragIdx, { x: pos.x, y: pos.y });
    draw();
  }

  function onPointerUp() {
    dragIdx = -1;
  }

  function animate() {
    frame = (frame + 1) % 256;
    draw();
    raf = requestAnimationFrame(animate);
  }

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag mesh vertices directly on the canvas.';
  sidebar.appendChild(hint);

  draw();
  raf = requestAnimationFrame(animate);

  return () => {
    cancelAnimationFrame(raf);
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
  };
}
