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

// Step 1: Bundle TypeScript into public/dist/
const result = await Bun.build({
  entrypoints: ['./src/main.ts'],
  outdir: './public/dist',
  splitting: true,
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

// Copy public/pkg/ (WASM — must already exist from build:wasm step)
const pkgDir = join(DEMO_DIR, "public", "pkg");
if (existsSync(pkgDir)) {
  cpSync(pkgDir, join(DIST_DIR, "public", "pkg"), { recursive: true });
  console.log("  Copied public/pkg/ (WASM)");
} else {
  console.warn("  WARNING: public/pkg/ not found — run build:wasm first!");
}

console.log(`\nBuild complete: site assembled in dist/`);
