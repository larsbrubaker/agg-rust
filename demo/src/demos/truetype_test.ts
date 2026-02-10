import { createDemoLayout, addSlider, addCheckbox, addRadioGroup, renderToCanvas } from '../render-canvas.ts';

export function init(container: HTMLElement) {
  const { canvas, sidebar, timeEl } = createDemoLayout(
    container,
    'TrueType LCD Subpixel',
    'LCD subpixel font rendering with faux weight/italic, gamma, and multiple typefaces. Port of C++ truetype_test_02_win.',
  );

  // Canvas size matching C++ app.init(640, 560)
  const W = 640, H = 560;

  // Parameter state (indices match Rust render fn)
  let typefaceIdx = 0;    // [0] 0=Serif, 1=Sans
  let fontScale = 1.0;    // [1] 0.5..2.0
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
    });
  }

  // Typeface radio group
  addRadioGroup(sidebar, 'Typeface', ['Liberation Serif', 'Liberation Sans'], 0, i => {
    typefaceIdx = i;
    draw();
  });

  // Sliders
  addSlider(sidebar, 'Font Scale', 0.5, 2.0, 1.0, 0.01, v => { fontScale = v; draw(); });
  addSlider(sidebar, 'Faux Italic', -1, 1, 0, 0.01, v => { fauxItalic = v; draw(); });
  addSlider(sidebar, 'Faux Weight', -1, 1, 0, 0.01, v => { fauxWeight = v; draw(); });
  addSlider(sidebar, 'Interval', -0.2, 0.2, 0, 0.001, v => { interval = v; draw(); });
  addSlider(sidebar, 'Width', 0.75, 1.25, 1.0, 0.01, v => { widthVal = v; draw(); });
  addSlider(sidebar, 'Gamma', 0.5, 2.5, 1.0, 0.01, v => { gamma = v; draw(); });
  addSlider(sidebar, 'Primary Weight', 0, 1, 1/3, 0.01, v => { primaryWt = v; draw(); });

  // Checkboxes
  addCheckbox(sidebar, 'Grayscale', false, v => { grayscale = v; draw(); });
  addCheckbox(sidebar, 'Hinting', true, v => { hinting = v; draw(); });
  addCheckbox(sidebar, 'Kerning', true, v => { kerning = v; draw(); });
  addCheckbox(sidebar, 'Invert', false, v => { invert = v; draw(); });

  draw();
}
