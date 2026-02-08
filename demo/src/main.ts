// Main entry point — SPA router and WASM initialization

import { initWasm } from './wasm.ts';

// Demo page modules (lazy loaded)
type DemoInit = (container: HTMLElement) => (() => void) | void;
const demoModules: Record<string, () => Promise<{ init: DemoInit }>> = {
  'lion': () => import('./demos/lion.ts'),
  'shapes': () => import('./demos/shapes.ts'),
  'gradients': () => import('./demos/gradients.ts'),
  'gouraud': () => import('./demos/gouraud.ts'),
  'strokes': () => import('./demos/strokes.ts'),
  'curves': () => import('./demos/curves.ts'),
};

let currentCleanup: (() => void) | null = null;

function getRoute(): string {
  const hash = window.location.hash.slice(2) || '';
  return hash || 'home';
}

function updateNav(route: string) {
  document.querySelectorAll('.nav-link').forEach(el => {
    const r = (el as HTMLElement).dataset.route;
    el.classList.toggle('active', r === route);
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
        <a href="#/strokes" class="feature-card">
          <span class="card-icon">&#9135;</span>
          <h3>Strokes</h3>
          <p>Line caps (butt, square, round) and joins (miter, round, bevel) with variable width.</p>
        </a>
        <a href="#/curves" class="feature-card">
          <span class="card-icon">&#8765;</span>
          <h3>Curves</h3>
          <p>Quadratic and cubic B&eacute;zier curves with control point visualization.</p>
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
            <div class="stat-value">731</div>
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
