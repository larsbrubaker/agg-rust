// Build script — bundles TypeScript demos and assembles the full site for deployment
// Usage: bun run build.ts
//
// Output: dist/ directory containing the complete deployable site:
//   dist/
//     index.html
//     styles/main.css
//     public/dist/  (bundled JS)
//     public/pkg/   (WASM — must be built separately via build:wasm first)

import { join } from "path";
import { mkdirSync, cpSync, existsSync, rmSync } from "fs";

const DEMO_DIR = import.meta.dir;
const DIST_DIR = join(DEMO_DIR, "dist");
const PUBLIC_DIST_DIR = join(DEMO_DIR, "public", "dist");

// Step 1: Bundle TypeScript into public/dist/
if (existsSync(PUBLIC_DIST_DIR)) {
  rmSync(PUBLIC_DIST_DIR, { recursive: true, force: true });
}
mkdirSync(PUBLIC_DIST_DIR, { recursive: true });

const result = await Bun.build({
  entrypoints: ['./src/main.ts'],
  outdir: './public/dist',
  // Bun 1.3 may keep outputs in memory unless write is explicit.
  // We need emitted files for both local dev server and GitHub Pages deploys.
  write: true,
  // Use a single bundle to avoid stale hashed chunk 404s on static hosting/CDN caches.
  splitting: false,
  format: 'esm',
  minify: false,
  sourcemap: 'external',
  target: 'browser',
});

if (!result.success) {
  console.error('Build failed:');
  for (const log of result.logs) {
    console.error(log);
  }
  process.exit(1);
}

console.log(`Bundled ${result.outputs.length} JS files to public/dist/`);

// Step 2: Assemble the dist/ directory for deployment
// Clean and recreate dist/
if (existsSync(DIST_DIR)) {
  rmSync(DIST_DIR, { recursive: true, force: true });
}
mkdirSync(DIST_DIR, { recursive: true });

// Copy index.html
cpSync(join(DEMO_DIR, "index.html"), join(DIST_DIR, "index.html"));
console.log("  Copied index.html");

// Copy styles/
cpSync(join(DEMO_DIR, "styles"), join(DIST_DIR, "styles"), { recursive: true });
console.log("  Copied styles/");

// Copy public/dist/ (bundled JS)
cpSync(join(DEMO_DIR, "public", "dist"), join(DIST_DIR, "public", "dist"), { recursive: true });
console.log("  Copied public/dist/ (JS bundles)");

// Copy public/thumbnails/ (demo screenshot thumbnails)
const thumbDir = join(DEMO_DIR, "public", "thumbnails");
if (existsSync(thumbDir)) {
  cpSync(thumbDir, join(DIST_DIR, "public", "thumbnails"), { recursive: true });
  console.log("  Copied public/thumbnails/");
} else {
  console.warn("  WARNING: public/thumbnails/ not found");
}

// Copy public/pkg/ (WASM — must already exist from build:wasm step)
const pkgDir = join(DEMO_DIR, "public", "pkg");
if (existsSync(pkgDir)) {
  cpSync(pkgDir, join(DIST_DIR, "public", "pkg"), { recursive: true });
  console.log("  Copied public/pkg/ (WASM)");
} else {
  console.warn("  WARNING: public/pkg/ not found — run build:wasm first!");
}

console.log(`\nBuild complete: site assembled in dist/`);
