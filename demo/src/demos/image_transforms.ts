import {
  createDemoLayout,
  addSlider,
  addCheckbox,
  addRadioGroup,
  renderToCanvas,
} from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Image Transforms',
    'Star polygon textured with image through 7 transform modes - matching C++ image_transforms.cpp.',
  );

  const W = 320;
  const H = 300;

  let polyAngle = 0.0;
  let polyScale = 1.0;
  let imgAngle = 0.0;
  let imgScale = 1.0;
  let rotatePolygon = false;
  let rotateImage = false;
  let exampleIdx = 0;
  let imageCx = W / 2;
  let imageCy = H / 2;
  let polygonCx = W / 2;
  let polygonCy = H / 2;

  let dragFlag = 0;
  let dx = 0;
  let dy = 0;
  let rafId = 0;

  function draw() {
    renderToCanvas({
      demoName: 'image_transforms',
      canvas,
      width: W,
      height: H,
      params: [
        polyAngle,
        polyScale,
        imgAngle,
        imgScale,
        rotatePolygon ? 1 : 0,
        rotateImage ? 1 : 0,
        exampleIdx,
        imageCx,
        imageCy,
        polygonCx,
        polygonCy,
      ],
      timeDisplay: timeEl,
    });
  }

  const slPolyAngle = addSlider(sidebar, 'Polygon Angle', -180, 180, polyAngle, 0.01, v => {
    polyAngle = v;
    draw();
  });
  const slPolyScale = addSlider(sidebar, 'Polygon Scale', 0.1, 5.0, polyScale, 0.01, v => {
    polyScale = v;
    draw();
  });
  const slImgAngle = addSlider(sidebar, 'Image Angle', -180, 180, imgAngle, 0.01, v => {
    imgAngle = v;
    draw();
  });
  const slImgScale = addSlider(sidebar, 'Image Scale', 0.1, 5.0, imgScale, 0.01, v => {
    imgScale = v;
    draw();
  });

  const cbRotatePolygon = addCheckbox(sidebar, 'Rotate Polygon', rotatePolygon, v => {
    rotatePolygon = v;
    draw();
  });
  const cbRotateImage = addCheckbox(sidebar, 'Rotate Image', rotateImage, v => {
    rotateImage = v;
    draw();
  });

  const radioEls = addRadioGroup(sidebar, 'Example', ['0', '1', '2', '3', '4', '5', '6'], 0, v => {
    exampleIdx = v;
    draw();
  });

  const controls: CanvasControl[] = [
    { type: 'slider', x1: 5, y1: 5, x2: 145, y2: 11, min: -180, max: 180, sidebarEl: slPolyAngle, onChange: v => { polyAngle = v; draw(); } },
    { type: 'slider', x1: 5, y1: 19, x2: 145, y2: 26, min: 0.1, max: 5.0, sidebarEl: slPolyScale, onChange: v => { polyScale = v; draw(); } },
    { type: 'slider', x1: 155, y1: 5, x2: 300, y2: 12, min: -180, max: 180, sidebarEl: slImgAngle, onChange: v => { imgAngle = v; draw(); } },
    { type: 'slider', x1: 155, y1: 19, x2: 300, y2: 26, min: 0.1, max: 5.0, sidebarEl: slImgScale, onChange: v => { imgScale = v; draw(); } },
    { type: 'checkbox', x1: 5, y1: 33, x2: 125, y2: 45, sidebarEl: cbRotatePolygon, onChange: v => { rotatePolygon = v; draw(); } },
    { type: 'checkbox', x1: 5, y1: 47, x2: 125, y2: 59, sidebarEl: cbRotateImage, onChange: v => { rotateImage = v; draw(); } },
    { type: 'radio', x1: 5, y1: 56, x2: 40, y2: 190, numItems: 7, sidebarEls: radioEls, onChange: v => { exampleIdx = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, controls, draw);

  function canvasPosAgg(e: PointerEvent): { x: number; y: number } {
    const rect = canvas.getBoundingClientRect();
    const scaleX = canvas.width / rect.width;
    const scaleY = canvas.height / rect.height;
    return {
      x: (e.clientX - rect.left) * scaleX,
      y: canvas.height - (e.clientY - rect.top) * scaleY,
    };
  }

  function transformedStarPoints(): Array<{ x: number; y: number }> {
    const points: Array<{ x: number; y: number }> = [];
    const r = Math.min(W, H);
    const r1 = r / 3 - 8;
    const r2 = r1 / 1.45;
    const a = polyAngle * Math.PI / 180;
    const ca = Math.cos(a);
    const sa = Math.sin(a);

    for (let i = 0; i < 14; i++) {
      const angle = Math.PI * 2 * i / 14 - Math.PI / 2;
      const rr = (i & 1) ? r1 : r2;
      const x0 = polygonCx + Math.cos(angle) * rr;
      const y0 = polygonCy + Math.sin(angle) * rr;
      const x = (x0 - polygonCx) * polyScale;
      const y = (y0 - polygonCy) * polyScale;
      points.push({
        x: x * ca - y * sa + polygonCx,
        y: x * sa + y * ca + polygonCy,
      });
    }
    return points;
  }

  function pointInPolygon(x: number, y: number, verts: Array<{ x: number; y: number }>): boolean {
    let inside = false;
    for (let i = 0, j = verts.length - 1; i < verts.length; j = i++) {
      const xi = verts[i].x;
      const yi = verts[i].y;
      const xj = verts[j].x;
      const yj = verts[j].y;
      const intersects = ((yi > y) !== (yj > y)) &&
        (x < (xj - xi) * (y - yi) / ((yj - yi) || 1e-12) + xi);
      if (intersects) inside = !inside;
    }
    return inside;
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    const pos = canvasPosAgg(e);
    if (Math.hypot(pos.x - imageCx, pos.y - imageCy) < 5.0) {
      dx = pos.x - imageCx;
      dy = pos.y - imageCy;
      dragFlag = 1;
      canvas.setPointerCapture(e.pointerId);
      return;
    }
    if (pointInPolygon(pos.x, pos.y, transformedStarPoints())) {
      dx = pos.x - polygonCx;
      dy = pos.y - polygonCy;
      dragFlag = 2;
      canvas.setPointerCapture(e.pointerId);
    }
  }

  function onPointerMove(e: PointerEvent) {
    if ((e.buttons & 1) === 0 || dragFlag === 0) return;
    const pos = canvasPosAgg(e);
    if (dragFlag === 1) {
      imageCx = pos.x - dx;
      imageCy = pos.y - dy;
      draw();
    } else if (dragFlag === 2) {
      polygonCx = pos.x - dx;
      polygonCy = pos.y - dy;
      draw();
    }
  }

  function onPointerUp() {
    dragFlag = 0;
  }

  function onTick() {
    if (rotatePolygon) {
      polyAngle += 0.5;
      if (polyAngle >= 180.0) polyAngle -= 360.0;
      slPolyAngle.value = String(polyAngle);
      slPolyAngle.dispatchEvent(new Event('input'));
    }
    if (rotateImage) {
      imgAngle += 0.5;
      if (imgAngle >= 180.0) imgAngle -= 360.0;
      slImgAngle.value = String(imgAngle);
      slImgAngle.dispatchEvent(new Event('input'));
    }
    rafId = requestAnimationFrame(onTick);
  }

  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointercancel', onPointerUp);
  rafId = requestAnimationFrame(onTick);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Drag the image center marker or star polygon; controls also work on the canvas.';
  sidebar.appendChild(hint);

  draw();
  return () => {
    cancelAnimationFrame(rafId);
    canvas.removeEventListener('pointerdown', onPointerDown);
    canvas.removeEventListener('pointermove', onPointerMove);
    canvas.removeEventListener('pointerup', onPointerUp);
    canvas.removeEventListener('pointercancel', onPointerUp);
    cleanupCC();
  };
}
