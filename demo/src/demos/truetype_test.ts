import { createDemoLayout, addSlider, addCheckbox, addRadioGroup, renderToCanvas } from '../render-canvas.ts';
import { setupCanvasControls, CanvasControl } from '../canvas-controls.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'TrueType LCD Subpixel',
    'LCD subpixel font rendering with faux weight/italic, gamma, and multiple typefaces. Port of C++ truetype_test_02_win.',
  );

  // Canvas size matching C++ app.init(640, 560)
  const W = 640, H = 560;

  // Parameter state (indices match Rust render fn)
  let typefaceIdx = 4;    // [0] 0=Arial, 1=Tahoma, 2=Verdana, 3=Times, 4=Georgia
  let fontScale = 1.43;   // [1] 0.5..2.0 (match C++ comparison setup)
  let fauxItalic = 0.0;   // [2] -1..1
  let fauxWeight = 0.0;   // [3] -1..1
  let interval = 0.0;     // [4] -0.2..0.2
  let widthVal = 1.0;     // [5] 0.75..1.25
  let gamma = 1.0;        // [6] 0.5..2.5
  let primaryWt = 1/3;    // [7] 0..1
  let grayscale = false;  // [8]
  let hinting = true;     // [9]
  let kerning = true;     // [10]
  let invert = false;     // [11]

  function draw() {
    renderToCanvas({
      demoName: 'truetype_test',
      canvas, width: W, height: H,
      params: [
        typefaceIdx,
        fontScale,
        fauxItalic,
        fauxWeight,
        interval,
        widthVal,
        gamma,
        primaryWt,
        grayscale ? 1 : 0,
        hinting ? 1 : 0,
        kerning ? 1 : 0,
        invert ? 1 : 0,
      ],
      timeDisplay: timeEl,
      flipY: true,
    });
  }

  // Typeface radio group
  const radioTypeface = addRadioGroup(sidebar, 'Typeface', ['Arial', 'Tahoma', 'Verdana', 'Times', 'Georgia'], 4, i => {
    typefaceIdx = i;
    draw();
  });

  // Sliders
  const slFontScale = addSlider(sidebar, 'Font Scale', 0.5, 2.0, 1.43, 0.01, v => { fontScale = v; draw(); });
  const slFauxItalic = addSlider(sidebar, 'Faux Italic', -1, 1, 0, 0.01, v => { fauxItalic = v; draw(); });
  const slFauxWeight = addSlider(sidebar, 'Faux Weight', -1, 1, 0, 0.01, v => { fauxWeight = v; draw(); });
  const slInterval = addSlider(sidebar, 'Interval', -0.2, 0.2, 0, 0.001, v => { interval = v; draw(); });
  const slWidth = addSlider(sidebar, 'Width', 0.75, 1.25, 1.0, 0.01, v => { widthVal = v; draw(); });
  const slGamma = addSlider(sidebar, 'Gamma', 0.5, 2.5, 1.0, 0.01, v => { gamma = v; draw(); });
  const slPrimaryWt = addSlider(sidebar, 'Primary Weight', 0, 1, 1/3, 0.01, v => { primaryWt = v; draw(); });

  // Checkboxes
  const cbGrayscale = addCheckbox(sidebar, 'Grayscale', false, v => { grayscale = v; draw(); });
  const cbHinting = addCheckbox(sidebar, 'Hinting', true, v => { hinting = v; draw(); });
  const cbKerning = addCheckbox(sidebar, 'Kerning', true, v => { kerning = v; draw(); });
  const cbInvert = addCheckbox(sidebar, 'Invert', false, v => { invert = v; draw(); });

  // Canvas controls mapped to the AGG-rendered controls.
  const canvasControls: CanvasControl[] = [
    { type: 'radio', x1: 5, y1: 5, x2: 155, y2: 110, numItems: 5, sidebarEls: radioTypeface, onChange: i => { typefaceIdx = i; draw(); } },
    { type: 'slider', x1: 160, y1: 10, x2: 635, y2: 17, min: 0.5, max: 2.0, sidebarEl: slFontScale, onChange: v => { fontScale = v; draw(); } },
    { type: 'slider', x1: 160, y1: 25, x2: 635, y2: 32, min: -1.0, max: 1.0, sidebarEl: slFauxItalic, onChange: v => { fauxItalic = v; draw(); } },
    { type: 'slider', x1: 160, y1: 40, x2: 635, y2: 47, min: -1.0, max: 1.0, sidebarEl: slFauxWeight, onChange: v => { fauxWeight = v; draw(); } },
    { type: 'slider', x1: 260, y1: 55, x2: 635, y2: 62, min: -0.2, max: 0.2, sidebarEl: slInterval, onChange: v => { interval = v; draw(); } },
    { type: 'slider', x1: 260, y1: 70, x2: 635, y2: 77, min: 0.75, max: 1.25, sidebarEl: slWidth, onChange: v => { widthVal = v; draw(); } },
    { type: 'slider', x1: 260, y1: 85, x2: 635, y2: 92, min: 0.5, max: 2.5, sidebarEl: slGamma, onChange: v => { gamma = v; draw(); } },
    { type: 'slider', x1: 260, y1: 100, x2: 635, y2: 107, min: 0.0, max: 1.0, sidebarEl: slPrimaryWt, onChange: v => { primaryWt = v; draw(); } },
    { type: 'checkbox', x1: 160, y1: 50, x2: 250, y2: 64, sidebarEl: cbGrayscale, onChange: v => { grayscale = v; draw(); } },
    { type: 'checkbox', x1: 160, y1: 65, x2: 250, y2: 79, sidebarEl: cbHinting, onChange: v => { hinting = v; draw(); } },
    { type: 'checkbox', x1: 160, y1: 80, x2: 250, y2: 94, sidebarEl: cbKerning, onChange: v => { kerning = v; draw(); } },
    { type: 'checkbox', x1: 160, y1: 95, x2: 250, y2: 109, sidebarEl: cbInvert, onChange: v => { invert = v; draw(); } },
  ];
  const cleanupCC = setupCanvasControls(canvas, canvasControls, draw, { origin: 'bottom-left' });

  draw();
  return () => cleanupCC();
}
