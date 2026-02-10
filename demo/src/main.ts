// Main entry point — SPA router and WASM initialization

import { initWasm } from './wasm.ts';

// Demo page modules (lazy loaded)
type DemoInit = (container: HTMLElement) => (() => void) | void;
const demoModules: Record<string, () => Promise<{ init: DemoInit }>> = {
  'lion': () => import('./demos/lion.ts'),
  'shapes': () => import('./demos/shapes.ts'),
  'gradients': () => import('./demos/gradients.ts'),
  'gouraud': () => import('./demos/gouraud.ts'),
  'conv_stroke': () => import('./demos/conv_stroke.ts'),
  'bezier_div': () => import('./demos/bezier_div.ts'),
  'circles': () => import('./demos/circles.ts'),
  'rounded_rect': () => import('./demos/rounded_rect.ts'),
  'aa_demo': () => import('./demos/aa_demo.ts'),
  'gamma_correction': () => import('./demos/gamma_correction.ts'),
  'line_thickness': () => import('./demos/line_thickness.ts'),
  'rasterizers': () => import('./demos/rasterizers.ts'),
  'conv_contour': () => import('./demos/conv_contour.ts'),
  'conv_dash': () => import('./demos/conv_dash.ts'),
  'gsv_text': () => import('./demos/gsv_text.ts'),
  'perspective': () => import('./demos/perspective.ts'),
  'image_fltr_graph': () => import('./demos/image_fltr_graph.ts'),
  'image1': () => import('./demos/image1.ts'),
  'image_filters': () => import('./demos/image_filters.ts'),
  'gradient_focal': () => import('./demos/gradient_focal.ts'),
  'idea': () => import('./demos/idea.ts'),
  'graph_test': () => import('./demos/graph_test.ts'),
  'gamma_tuner': () => import('./demos/gamma_tuner.ts'),
  'image_filters2': () => import('./demos/image_filters2.ts'),
  'conv_dash_marker': () => import('./demos/conv_dash_marker.ts'),
  'aa_test': () => import('./demos/aa_test.ts'),
  'bspline': () => import('./demos/bspline.ts'),
  'image_perspective': () => import('./demos/image_perspective.ts'),
  'alpha_mask': () => import('./demos/alpha_mask.ts'),
  'alpha_gradient': () => import('./demos/alpha_gradient.ts'),
  'image_alpha': () => import('./demos/image_alpha.ts'),
  'alpha_mask3': () => import('./demos/alpha_mask3.ts'),
  'image_transforms': () => import('./demos/image_transforms.ts'),
  'mol_view': () => import('./demos/mol_view.ts'),
  'raster_text': () => import('./demos/raster_text.ts'),
  'gamma_ctrl': () => import('./demos/gamma_ctrl.ts'),
  'trans_polar': () => import('./demos/trans_polar.ts'),
  'multi_clip': () => import('./demos/multi_clip.ts'),
  'simple_blur': () => import('./demos/simple_blur.ts'),
  'blur': () => import('./demos/blur.ts'),
  'trans_curve1': () => import('./demos/trans_curve1.ts'),
  'trans_curve2': () => import('./demos/trans_curve2.ts'),
  'lion_lens': () => import('./demos/lion_lens.ts'),
  'distortions': () => import('./demos/distortions.ts'),
  'blend_color': () => import('./demos/blend_color.ts'),
  'component_rendering': () => import('./demos/component_rendering.ts'),
  'polymorphic_renderer': () => import('./demos/polymorphic_renderer.ts'),
  'scanline_boolean': () => import('./demos/scanline_boolean.ts'),
  'scanline_boolean2': () => import('./demos/scanline_boolean2.ts'),
  'pattern_fill': () => import('./demos/pattern_fill.ts'),
  'pattern_perspective': () => import('./demos/pattern_perspective.ts'),
  'pattern_resample': () => import('./demos/pattern_resample.ts'),
  'lion_outline': () => import('./demos/lion_outline.ts'),
  'rasterizers2': () => import('./demos/rasterizers2.ts'),
  'line_patterns': () => import('./demos/line_patterns.ts'),
  'line_patterns_clip': () => import('./demos/line_patterns_clip.ts'),
  'compositing': () => import('./demos/compositing.ts'),
  'compositing2': () => import('./demos/compositing2.ts'),
  'flash_rasterizer': () => import('./demos/flash_rasterizer.ts'),
  'flash_rasterizer2': () => import('./demos/flash_rasterizer2.ts'),
  'rasterizer_compound': () => import('./demos/rasterizer_compound.ts'),
  'gouraud_mesh': () => import('./demos/gouraud_mesh.ts'),
  'image_resample': () => import('./demos/image_resample.ts'),
  'alpha_mask2': () => import('./demos/alpha_mask2.ts'),
};

// Mapping of demo route name to thumbnail image filename
const thumbnails: Record<string, string> = {
  'aa_demo': 'aa_demo_s.gif',
  'aa_test': 'aa_test_s.png',
  'alpha_gradient': 'alpha_gradient_s.png',
  'alpha_mask': 'alpha_mask_s.gif',
  'alpha_mask2': 'alpha_mask2_s.jpg',
  'alpha_mask3': 'alpha_mask3_s.gif',
  'bezier_div': 'bezier_div_s.png',
  'blend_color': 'compositing_s.png',
  'blur': 'blur_s.png',
  'bspline': 'bezier_div_s.png',
  'circles': 'circles_s.gif',
  'component_rendering': 'component_rendering_s.gif',
  'compositing': 'compositing_s.png',
  'compositing2': 'compositing2_s.png',
  'conv_contour': 'conv_contour_s.gif',
  'conv_dash': 'conv_dash_marker_s.gif',
  'conv_dash_marker': 'conv_dash_marker_s.gif',
  'conv_stroke': 'conv_stroke_s.gif',
  'distortions': 'distortions_s.png',
  'flash_rasterizer': 'flash_rasterizer_s.png',
  'flash_rasterizer2': 'flash_rasterizer2_s.png',
  'gamma_correction': 'gamma_correction_s.gif',
  'gamma_ctrl': 'gamma_ctrl_s.gif',
  'gamma_tuner': 'gamma_tuner_s.png',
  'gouraud': 'gouraud_s.png',
  'gouraud_mesh': 'gouraud_mesh_s.png',
  'gradient_focal': 'gradient_focal_s.png',
  'gradients': 'gradients_s.png',
  'graph_test': 'graph_test_s.gif',
  'gsv_text': 'raster_text_s.gif',
  'idea': 'idea_s.gif',
  'image_alpha': 'image_alpha_s.png',
  'image_filters': 'image_filters_s.jpg',
  'image_filters2': 'image_filters2_s.png',
  'image_fltr_graph': 'image_fltr_graph_s.gif',
  'image_perspective': 'image_perspective_s.jpg',
  'image_resample': 'image_resample_s.jpg',
  'image_transforms': 'image_transforms_s.jpg',
  'image1': 'image1_s.jpg',
  'line_patterns': 'line_patterns_s.gif',
  'line_patterns_clip': 'line_patterns_clip_s.png',
  'line_thickness': 'conv_stroke_s.gif',
  'lion': 'lion_s.png',
  'lion_lens': 'lion_lens_s.gif',
  'lion_outline': 'lion_outline_s.gif',
  'mol_view': 'mol_view_s.gif',
  'multi_clip': 'multi_clip_s.gif',
  'pattern_fill': 'pattern_fill_s.gif',
  'pattern_perspective': 'pattern_perspective_s.jpg',
  'pattern_resample': 'pattern_resample_s.jpg',
  'perspective': 'perspective_s.gif',
  'polymorphic_renderer': 'polymorphic_renderer_s.gif',
  'raster_text': 'raster_text_s.gif',
  'rasterizer_compound': 'rasterizer_compound_s.png',
  'rasterizers': 'rasterizers_s.gif',
  'rasterizers2': 'rasterizers2_s.gif',
  'rounded_rect': 'rounded_rect_s.gif',
  'scanline_boolean': 'scanline_boolean_s.gif',
  'scanline_boolean2': 'scanline_boolean2_s.gif',
  'shapes': 'circles_s.gif',
  'simple_blur': 'simple_blur_s.gif',
  'trans_curve1': 'trans_curve1_s.gif',
  'trans_curve2': 'trans_curve2_s.gif',
  'trans_polar': 'trans_polar_s.gif',
};

/** Returns an <img> tag for the demo thumbnail, or a fallback icon span */
function thumbImg(route: string, cssClass: string): string {
  const file = thumbnails[route];
  if (file) {
    return `<img class="${cssClass}" src="./public/thumbnails/${file}" alt="${route}" loading="lazy">`;
  }
  return `<span class="${cssClass === 'card-thumb' ? 'card-thumb-fallback' : 'nav-icon'}">&#9670;</span>`;
}

let currentCleanup: (() => void) | null = null;

function getRoute(): string {
  const hash = window.location.hash.slice(2) || '';
  return hash || 'home';
}

function updateNav(route: string) {
  document.querySelectorAll('.nav-link').forEach(el => {
    const r = (el as HTMLElement).dataset.route;
    const isActive = r === route;
    el.classList.toggle('active', isActive);
    // Auto-expand the section containing the active link
    if (isActive) {
      const group = el.closest('.nav-group');
      if (group) {
        group.classList.add('open');
        const btn = group.querySelector('.nav-section');
        if (btn) btn.setAttribute('aria-expanded', 'true');
        // Persist to localStorage
        const KEY = 'agg-sidebar-sections';
        try {
          const saved = JSON.parse(localStorage.getItem(KEY) || '{}');
          saved[(group as HTMLElement).dataset.section!] = true;
          localStorage.setItem(KEY, JSON.stringify(saved));
        } catch(e) {}
      }
    }
  });
}

// Demo card definitions for the home page grid
const demoCards: Array<{ route: string; title: string; desc: string }> = [
  { route: 'lion', title: 'Lion', desc: 'The classic AGG lion &mdash; a complex vector graphic with rotation and scaling controls.' },
  { route: 'shapes', title: 'Shapes', desc: 'Anti-aliased circles, ellipses, and rounded rectangles at various sizes and colors.' },
  { route: 'gradients', title: 'Gradients', desc: 'Linear and radial gradient fills with multi-stop color interpolation.' },
  { route: 'gouraud', title: 'Gouraud Shading', desc: 'Smooth color interpolation across triangles using Gouraud shading.' },
  { route: 'conv_stroke', title: 'Conv Stroke', desc: 'Line joins (miter, round, bevel), caps, and dashed overlay with draggable vertices.' },
  { route: 'bezier_div', title: 'Bezier Div', desc: 'Cubic B&eacute;zier curve subdivision with draggable control points and width control.' },
  { route: 'circles', title: 'Circles', desc: 'Random anti-aliased circles with configurable count, size range, and seed.' },
  { route: 'rounded_rect', title: 'Rounded Rect', desc: 'Draggable rounded rectangle with adjustable corner radius.' },
  { route: 'aa_demo', title: 'AA Demo', desc: 'Anti-aliasing visualization &mdash; enlarged pixel view of a triangle.' },
  { route: 'gamma_correction', title: 'Gamma Correction', desc: 'Gamma curve visualization with concentric colored ellipses.' },
  { route: 'line_thickness', title: 'Line Thickness', desc: 'Lines at varying sub-pixel widths from 0.1 to 5.0 pixels.' },
  { route: 'rasterizers', title: 'Rasterizers', desc: 'Filled and stroked triangle with alpha control.' },
  { route: 'conv_contour', title: 'Conv Contour', desc: 'Letter "A" with adjustable contour width and orientation control.' },
  { route: 'conv_dash', title: 'Conv Dash', desc: 'Dashed stroke patterns with cap styles on a draggable triangle.' },
  { route: 'gsv_text', title: 'GSV Text', desc: 'Built-in vector text engine with adjustable size and stroke width.' },
  { route: 'perspective', title: 'Perspective', desc: 'Lion with bilinear/perspective quad transform &mdash; drag corners to warp.' },
  { route: 'image_fltr_graph', title: 'Filter Graph', desc: 'Image filter kernel weight function visualization &mdash; 16 filters.' },
  { route: 'image1', title: 'Image Transforms', desc: 'Procedural sphere image with affine rotation/scaling through a bilinear filter.' },
  { route: 'image_filters', title: 'Image Filters', desc: 'Iterative rotation showing filter quality degradation &mdash; 17 filter types.' },
  { route: 'gradient_focal', title: 'Gradient Focal', desc: 'Radial gradient with moveable focal point and reflect adaptor.' },
  { route: 'idea', title: 'Idea', desc: 'Rotating light bulb icon with even-odd fill, draft, and roundoff options.' },
  { route: 'graph_test', title: 'Graph Test', desc: 'Random graph with 200 nodes and 100 edges &mdash; 5 rendering modes.' },
  { route: 'gamma_tuner', title: 'Gamma Tuner', desc: 'Gradient background with alpha pattern and gamma correction controls.' },
  { route: 'image_filters2', title: 'Image Filters 2', desc: '4x4 test image filtered through 17 filter types with graph visualization.' },
  { route: 'conv_dash_marker', title: 'Dash Marker', desc: 'Dashed strokes with cap styles on a draggable triangle.' },
  { route: 'aa_test', title: 'AA Test', desc: 'Anti-aliasing quality test &mdash; radial lines, gradient lines, Gouraud triangles.' },
  { route: 'bspline', title: 'B-Spline', desc: 'B-spline curve through 6 draggable control points with adjustable density.' },
  { route: 'image_perspective', title: 'Image Perspective', desc: 'Image transformed through affine/bilinear/perspective quad corners.' },
  { route: 'alpha_mask', title: 'Alpha Mask', desc: 'Lion with elliptical alpha mask &mdash; rotate, scale, and skew.' },
  { route: 'alpha_gradient', title: 'Alpha Gradient', desc: 'Gradient with alpha curve control over random ellipse background.' },
  { route: 'image_alpha', title: 'Image Alpha', desc: 'Image with brightness-to-alpha mapping over random ellipses.' },
  { route: 'alpha_mask3', title: 'Alpha Mask 3', desc: 'Alpha mask polygon clipping with AND/SUB operations.' },
  { route: 'image_transforms', title: 'Image Transforms', desc: 'Star polygon textured with image through 7 transform modes.' },
  { route: 'mol_view', title: 'Molecule Viewer', desc: 'Molecular structure viewer with rotate, scale, and pan controls.' },
  { route: 'raster_text', title: 'Raster Text', desc: 'All 34 embedded bitmap fonts rendered with sample text strings.' },
  { route: 'gamma_ctrl', title: 'Gamma Control', desc: 'Interactive gamma spline widget with stroked ellipses.' },
  { route: 'trans_polar', title: 'Polar Transform', desc: 'Slider control warped through polar coordinates with spiral effect.' },
  { route: 'multi_clip', title: 'Multi Clip', desc: 'Lion rendered through N&times;N clip regions with random shapes.' },
  { route: 'simple_blur', title: 'Simple Blur', desc: 'Lion with 3&times;3 box blur &mdash; original vs blurred comparison.' },
  { route: 'blur', title: 'Blur', desc: 'Stack blur and recursive blur on colored shapes with adjustable radius.' },
  { route: 'trans_curve1', title: 'Text on Curve', desc: 'Text warped along a B-spline curve with draggable control points.' },
  { route: 'trans_curve2', title: 'Text on Curve 2', desc: 'Text warped along a curve with adjustable approximation scale.' },
  { route: 'lion_lens', title: 'Lion Lens', desc: 'Magnifying lens distortion on the lion using trans_warp_magnifier.' },
  { route: 'distortions', title: 'Distortions', desc: 'Wave and swirl distortions on a procedural image with adjustable parameters.' },
  { route: 'blend_color', title: 'Blend Color', desc: 'Color blending modes with alpha compositing demonstration.' },
  { route: 'component_rendering', title: 'Component Rendering', desc: 'Per-component rendering of individual color channels.' },
  { route: 'polymorphic_renderer', title: 'Polymorphic Renderer', desc: 'Multiple renderer types dispatched through a common interface.' },
  { route: 'scanline_boolean', title: 'Scanline Boolean', desc: 'Boolean operations (AND, OR, XOR, SUB) on scanline shapes.' },
  { route: 'scanline_boolean2', title: 'Scanline Boolean 2', desc: 'Advanced boolean polygon operations with multiple shapes.' },
  { route: 'pattern_fill', title: 'Pattern Fill', desc: 'Tiled pattern fill on polygon shapes.' },
  { route: 'pattern_perspective', title: 'Pattern Perspective', desc: 'Pattern fill with perspective transformation.' },
  { route: 'pattern_resample', title: 'Pattern Resample', desc: 'Pattern resampling with various filter types.' },
  { route: 'lion_outline', title: 'Lion Outline', desc: 'Lion rendered as stroked outlines with adjustable width.' },
  { route: 'rasterizers2', title: 'Rasterizers 2', desc: 'Extended rasterizer comparison with outline and gamma controls.' },
  { route: 'line_patterns', title: 'Line Patterns', desc: 'Custom line patterns with clip regions.' },
  { route: 'line_patterns_clip', title: 'Line Patterns Clip', desc: 'Line patterns with clipping rectangle.' },
  { route: 'compositing', title: 'Compositing', desc: 'Porter-Duff compositing operators visualization.' },
  { route: 'compositing2', title: 'Compositing 2', desc: 'Advanced compositing with alpha blending modes.' },
  { route: 'flash_rasterizer', title: 'Flash Rasterizer', desc: 'Flash-style compound shape rasterization.' },
  { route: 'flash_rasterizer2', title: 'Flash Rasterizer 2', desc: 'Extended Flash-style rasterizer with styles.' },
  { route: 'rasterizer_compound', title: 'Compound Rasterizer', desc: 'Compound shape rasterizer with style handler.' },
  { route: 'gouraud_mesh', title: 'Gouraud Mesh', desc: 'Triangle mesh with Gouraud shading interpolation.' },
  { route: 'image_resample', title: 'Image Resample', desc: 'Image resampling with perspective transform and filter comparison.' },
  { route: 'alpha_mask2', title: 'Alpha Mask 2', desc: 'Alpha mask with gray8 rendering buffer.' },
];

function renderHome(container: HTMLElement) {
  const cardsHtml = demoCards.map(card => `
        <a href="#/${card.route}" class="feature-card">
          ${thumbImg(card.route, 'card-thumb')}
          <h3>${card.title}</h3>
          <p>${card.desc}</p>
        </a>`).join('');

  container.innerHTML = `
    <div class="home-page">
      <div class="github-badge">
        <a href="https://github.com/larsbrubaker/agg-rust" target="_blank" class="github-badge-link">
          <svg height="20" viewBox="0 0 16 16" width="20" fill="currentColor"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/></svg>
          <span>larsbrubaker/agg-rust</span>
        </a>
      </div>
      <div class="hero">
        <h1>AGG <span>for Rust</span></h1>
        <p>
          A pure Rust port of Anti-Grain Geometry (AGG) 2.6 &mdash; the legendary
          high-quality 2D software rendering library. Explore interactive demos
          showcasing anti-aliased rendering, gradient fills, Gouraud shading,
          and more &mdash; all running in your browser via WebAssembly.
        </p>
      </div>
      <div class="feature-grid">${cardsHtml}
      </div>
      <div class="about-section">
        <h2>About This Project</h2>
        <p>
          This is a pure Rust port of Maxim Shemanarev's
          <a href="http://www.antigrain.com" target="_blank">Anti-Grain Geometry</a>
          C++ library (version 2.6). AGG is a software rendering engine that
          produces pixel-perfect anti-aliased output without relying on any
          GPU or platform graphics API.
        </p>
        <p style="margin-top: 12px">
          Ported by <strong>Lars Brubaker</strong>, sponsored by
          <a href="https://www.matterhackers.com" target="_blank">MatterHackers</a>.
        </p>
        <div class="stats-row">
          <div class="stat">
            <div class="stat-value">88</div>
            <div class="stat-label">Modules Ported</div>
          </div>
          <div class="stat">
            <div class="stat-value">903</div>
            <div class="stat-label">Tests Passing</div>
          </div>
          <div class="stat">
            <div class="stat-value">100%</div>
            <div class="stat-label">Software Rendered</div>
          </div>
          <div class="stat">
            <div class="stat-value">0</div>
            <div class="stat-label">GPU Dependencies</div>
          </div>
        </div>
      </div>
    </div>
  `;
}

async function navigate(route: string) {
  const container = document.getElementById('main-content')!;

  // Cleanup previous demo
  if (currentCleanup) {
    currentCleanup();
    currentCleanup = null;
  }

  updateNav(route);

  if (route === 'home') {
    renderHome(container);
    return;
  }

  const loader = demoModules[route];
  if (!loader) {
    container.innerHTML = `<div class="home-page"><h2>Page not found</h2><p>Unknown route: ${route}</p></div>`;
    return;
  }

  container.innerHTML = `<div class="home-page" style="display:flex;align-items:center;justify-content:center;height:80vh;"><p style="color:var(--text-muted)">Loading demo...</p></div>`;

  try {
    await initWasm();
    const mod = await loader();
    container.innerHTML = '';
    const cleanup = mod.init(container);
    if (cleanup) currentCleanup = cleanup;
  } catch (e) {
    console.error('Failed to load demo:', e);
    container.innerHTML = `<div class="home-page"><h2>Error loading demo</h2><pre style="color:var(--accent)">${e}</pre></div>`;
  }
}

// Route on hash change
window.addEventListener('hashchange', () => navigate(getRoute()));

// Initial load
navigate(getRoute());
