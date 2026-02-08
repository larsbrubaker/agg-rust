import { createDemoLayout, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'Image Transforms',
    'Star polygon textured with image through 7 transform modes — matching C++ image_transforms.cpp.',
  );

  const W = 430, H = 340;

  let polyAngle = 0.0;
  let polyScale = 1.0;
  let imgAngle = 0.0;
  let imgScale = 1.0;
  let exampleIdx = 1;

  function draw() {
    renderToCanvas({
      demoName: 'image_transforms',
      canvas, width: W, height: H,
      params: [polyAngle, polyScale, imgAngle, imgScale, exampleIdx, W / 2, H / 2, W / 2, H / 2],
      timeDisplay: timeEl,
    });
  }

  // Example radio buttons
  const radioDiv = document.createElement('div');
  radioDiv.className = 'control-group';
  const radioLabel = document.createElement('label');
  radioLabel.className = 'control-label';
  radioLabel.textContent = 'Transform Example';
  radioDiv.appendChild(radioLabel);
  const names = [
    '1: Rotate around (img_cx, img_cy)',
    '2: Plus translate to center',
    '3: Image in polygon coords',
    '4: Image in polygon + rotate',
    '5: Image in polygon + rotate + scale',
    '6: Rotate image + polygon same center',
    '7: Rotate image + polygon separately',
  ];
  names.forEach((name, i) => {
    const row = document.createElement('label');
    row.style.display = 'block';
    row.style.cursor = 'pointer';
    row.style.marginBottom = '2px';
    row.style.fontSize = '12px';
    const rb = document.createElement('input');
    rb.type = 'radio';
    rb.name = 'img_trans_example';
    rb.value = String(i + 1);
    rb.checked = (i + 1) === exampleIdx;
    rb.addEventListener('change', () => { exampleIdx = i + 1; draw(); });
    row.appendChild(rb);
    row.appendChild(document.createTextNode(' ' + name));
    radioDiv.appendChild(row);
  });
  sidebar.appendChild(radioDiv);

  const controls: CanvasControl[] = [
    {
      type: 'slider',
      label: 'Polygon Angle',
      min: -180, max: 180, step: 1,
      initial: polyAngle,
      onChange(v) { polyAngle = v; draw(); },
    },
    {
      type: 'slider',
      label: 'Polygon Scale',
      min: 0.1, max: 5.0, step: 0.05,
      initial: polyScale,
      onChange(v) { polyScale = v; draw(); },
    },
    {
      type: 'slider',
      label: 'Image Angle',
      min: -180, max: 180, step: 1,
      initial: imgAngle,
      onChange(v) { imgAngle = v; draw(); },
    },
    {
      type: 'slider',
      label: 'Image Scale',
      min: 0.1, max: 5.0, step: 0.05,
      initial: imgScale,
      onChange(v) { imgScale = v; draw(); },
    },
  ];
  const cleanupCC = setupCanvasControls(canvas, controls, draw);

  const hint = document.createElement('div');
  hint.className = 'control-hint';
  hint.textContent = 'Select transform example and adjust angles/scales.';
  sidebar.appendChild(hint);

  draw();
  return () => { cleanupCC(); };
}
