import { createDemoLayout, addSlider, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container, 'Flash Rasterizer 2',
    'Multi-style shapes rendered with regular rasterizer — matching C++ flash_rasterizer2.cpp behavior.',
  );
  const W = 655, H = 520;
  let scale = 1.0;
  let rotation = 0;
  let shapeIndex = 0;

  function draw() {
    renderToCanvas({ demoName: 'flash_rasterizer2', canvas, width: W, height: H,
      params: [scale, rotation, shapeIndex], timeDisplay: timeEl });
  }

  const slScale = addSlider(sidebar, 'Scale', 0.2, 3, 1, 0.01, v => { scale = v; draw(); });
  const slRotation = addSlider(sidebar, 'Rotation', -180, 180, 0, 1, v => { rotation = v; draw(); });

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === ' ') {
      shapeIndex += 1;
      draw();
      e.preventDefault();
      return;
    }
    if (e.key === '+' || e.key === '=') {
      scale = Math.min(3, scale * 1.1);
      slScale.value = String(scale);
      slScale.dispatchEvent(new Event('input'));
      e.preventDefault();
      return;
    }
    if (e.key === '-') {
      scale = Math.max(0.2, scale / 1.1);
      slScale.value = String(scale);
      slScale.dispatchEvent(new Event('input'));
      e.preventDefault();
      return;
    }
    if (e.key === 'ArrowLeft') {
      rotation -= 9;
      slRotation.value = String(rotation);
      slRotation.dispatchEvent(new Event('input'));
      e.preventDefault();
      return;
    }
    if (e.key === 'ArrowRight') {
      rotation += 9;
      slRotation.value = String(rotation);
      slRotation.dispatchEvent(new Event('input'));
      e.preventDefault();
    }
  }

  window.addEventListener('keydown', onKeyDown);
  draw();
  return () => {
    window.removeEventListener('keydown', onKeyDown);
  };
}
