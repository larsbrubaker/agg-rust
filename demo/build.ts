// Build script — bundles TypeScript demos for the browser
// Usage: bun run build.ts

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

console.log(`Build complete: ${result.outputs.length} files written to public/dist/`);
