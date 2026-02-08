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
};

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

function renderHome(container: HTMLElement) {
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
      <div class="feature-grid">
        <a href="#/lion" class="feature-card">
          <span class="card-icon">&#129409;</span>
          <h3>Lion</h3>
          <p>The classic AGG lion &mdash; a complex vector graphic with rotation and scaling controls.</p>
        </a>
        <a href="#/shapes" class="feature-card">
          <span class="card-icon">&#9711;</span>
          <h3>Shapes</h3>
          <p>Anti-aliased circles, ellipses, and rounded rectangles at various sizes and colors.</p>
        </a>
        <a href="#/gradients" class="feature-card">
          <span class="card-icon">&#9632;</span>
          <h3>Gradients</h3>
          <p>Linear and radial gradient fills with multi-stop color interpolation.</p>
        </a>
        <a href="#/gouraud" class="feature-card">
          <span class="card-icon">&#9650;</span>
          <h3>Gouraud Shading</h3>
          <p>Smooth color interpolation across triangles using Gouraud shading.</p>
        </a>
        <a href="#/conv_stroke" class="feature-card">
          <span class="card-icon">&#9135;</span>
          <h3>Conv Stroke</h3>
          <p>Line joins (miter, round, bevel), caps, and dashed overlay with draggable vertices.</p>
        </a>
        <a href="#/bezier_div" class="feature-card">
          <span class="card-icon">&#8765;</span>
          <h3>Bezier Div</h3>
          <p>Cubic B&eacute;zier curve subdivision with draggable control points and width control.</p>
        </a>
        <a href="#/circles" class="feature-card">
          <span class="card-icon">&#9679;</span>
          <h3>Circles</h3>
          <p>Random anti-aliased circles with configurable count, size range, and seed.</p>
        </a>
        <a href="#/rounded_rect" class="feature-card">
          <span class="card-icon">&#9645;</span>
          <h3>Rounded Rect</h3>
          <p>Draggable rounded rectangle with adjustable corner radius.</p>
        </a>
        <a href="#/aa_demo" class="feature-card">
          <span class="card-icon">&#9638;</span>
          <h3>AA Demo</h3>
          <p>Anti-aliasing visualization &mdash; enlarged pixel view of a triangle.</p>
        </a>
        <a href="#/gamma_correction" class="feature-card">
          <span class="card-icon">&#947;</span>
          <h3>Gamma Correction</h3>
          <p>Gamma curve visualization with concentric colored ellipses.</p>
        </a>
        <a href="#/line_thickness" class="feature-card">
          <span class="card-icon">&#8212;</span>
          <h3>Line Thickness</h3>
          <p>Lines at varying sub-pixel widths from 0.1 to 5.0 pixels.</p>
        </a>
        <a href="#/rasterizers" class="feature-card">
          <span class="card-icon">&#9651;</span>
          <h3>Rasterizers</h3>
          <p>Filled and stroked triangle with alpha control.</p>
        </a>
        <a href="#/conv_contour" class="feature-card">
          <span class="card-icon">&#9674;</span>
          <h3>Conv Contour</h3>
          <p>Letter "A" with adjustable contour width and orientation control.</p>
        </a>
        <a href="#/conv_dash" class="feature-card">
          <span class="card-icon">&#9473;</span>
          <h3>Conv Dash</h3>
          <p>Dashed stroke patterns with cap styles on a draggable triangle.</p>
        </a>
        <a href="#/gsv_text" class="feature-card">
          <span class="card-icon">A</span>
          <h3>GSV Text</h3>
          <p>Built-in vector text engine with adjustable size and stroke width.</p>
        </a>
        <a href="#/perspective" class="feature-card">
          <span class="card-icon">&#9670;</span>
          <h3>Perspective</h3>
          <p>Lion with bilinear/perspective quad transform &mdash; drag corners to warp.</p>
        </a>
        <a href="#/image_fltr_graph" class="feature-card">
          <span class="card-icon">&#8767;</span>
          <h3>Filter Graph</h3>
          <p>Image filter kernel weight function visualization &mdash; 16 filters.</p>
        </a>
        <a href="#/image1" class="feature-card">
          <span class="card-icon">&#127912;</span>
          <h3>Image Transforms</h3>
          <p>Procedural sphere image with affine rotation/scaling through a bilinear filter.</p>
        </a>
        <a href="#/image_filters" class="feature-card">
          <span class="card-icon">&#128247;</span>
          <h3>Image Filters</h3>
          <p>Iterative rotation showing filter quality degradation &mdash; 17 filter types.</p>
        </a>
        <a href="#/gradient_focal" class="feature-card">
          <span class="card-icon">&#9737;</span>
          <h3>Gradient Focal</h3>
          <p>Radial gradient with moveable focal point and reflect adaptor.</p>
        </a>
        <a href="#/idea" class="feature-card">
          <span class="card-icon">&#128161;</span>
          <h3>Idea</h3>
          <p>Rotating light bulb icon with even-odd fill, draft, and roundoff options.</p>
        </a>
        <a href="#/graph_test" class="feature-card">
          <span class="card-icon">&#128200;</span>
          <h3>Graph Test</h3>
          <p>Random graph with 200 nodes and 100 edges — 5 rendering modes.</p>
        </a>
        <a href="#/gamma_tuner" class="feature-card">
          <span class="card-icon">&#947;</span>
          <h3>Gamma Tuner</h3>
          <p>Gradient background with alpha pattern and gamma correction controls.</p>
        </a>
        <a href="#/image_filters2" class="feature-card">
          <span class="card-icon">&#128247;</span>
          <h3>Image Filters 2</h3>
          <p>4x4 test image filtered through 17 filter types with graph visualization.</p>
        </a>
        <a href="#/conv_dash_marker" class="feature-card">
          <span class="card-icon">&#10230;</span>
          <h3>Dash Marker</h3>
          <p>Dashed strokes with cap styles on a draggable triangle.</p>
        </a>
        <a href="#/aa_test" class="feature-card">
          <span class="card-icon">&#9646;</span>
          <h3>AA Test</h3>
          <p>Anti-aliasing quality test — radial lines, gradient lines, Gouraud triangles.</p>
        </a>
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
            <div class="stat-value">61</div>
            <div class="stat-label">Modules Ported</div>
          </div>
          <div class="stat">
            <div class="stat-value">742</div>
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
