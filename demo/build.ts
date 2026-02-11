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
import {
  mkdirSync,
  cpSync,
  existsSync,
  rmSync,
  readFileSync,
  writeFileSync,
  statSync,
  lstatSync,
  readdirSync,
} from "fs";
import {
  HISTORY_SECTIONS,
  HISTORY_SECTION_BY_SOURCE_PATH,
  historyRouteForSourcePath,
  isLocallyHostedHistorySourcePath,
} from "./src/legacy/sections.ts";

const DEMO_DIR = import.meta.dir;
const DIST_DIR = join(DEMO_DIR, "dist");
const PUBLIC_DIST_DIR = join(DEMO_DIR, "public", "dist");
const HISTORY_SOURCE_DIR = join(DEMO_DIR, "..", "cpp-references", "agg-web");
const HISTORY_PUBLIC_DIR = join(DEMO_DIR, "public", "history");
const HISTORY_FRAGMENTS_DIR = join(HISTORY_PUBLIC_DIR, "fragments");
const HISTORY_ASSETS_DIR = join(HISTORY_PUBLIC_DIR, "assets");

type HistoryIndexEntry = {
  route: string;
  sourcePath: string;
  sourceUrl: string;
  title: string;
  contentPath: string;
  generatedAt: string;
};

const HTML_EXT_RE = /\.(html|agdoc\.html)$/i;

function slash(path: string): string {
  return path.replaceAll("\\", "/");
}

function sourceUrl(path: string): string {
  return `https://agg.sourceforge.net/antigrain.com/${path}`;
}

function stripLegacyChrome(html: string): string {
  const afterHead = html.split(/<\/head>/i)[1] ?? html;
  const startTable = afterHead.lastIndexOf("<TABLE", afterHead.search(/<H1/i));
  const start = startTable >= 0 ? startTable : 0;

  const footerIndex = afterHead.search(
    /<TABLE width="640px" border="0" cellspacing="0" cellpadding="0">\s*<TR><TD><CENTER><SPAN class="authors">/i,
  );
  const end = footerIndex > start ? footerIndex : afterHead.length;

  return afterHead
    .slice(start, end)
    .replace(/<A name="[^"]*"><B><\/B><\/A>/gi, "")
    .replace(/(?:<BR\/?>\s*){10,}/gi, "<BR/><BR/>");
}

function rewriteLinksAndCollectAssets(
  html: string,
  sourcePath: string,
  assetPaths: Set<string>,
): string {
  const sourceDir = sourcePath.includes("/")
    ? sourcePath.slice(0, sourcePath.lastIndexOf("/") + 1)
    : "";

  return html.replace(
    /\b(href|src)=["']([^"']+)["']/gi,
    (_all, attr: string, original: string) => {
      const value = original.trim();
      if (
        value.startsWith("#") ||
        value.startsWith("mailto:") ||
        value.startsWith("javascript:") ||
        /^[a-z]+:/i.test(value)
      ) {
        return `${attr}="${value}"`;
      }

      const base = new URL(`https://history.local/${sourceDir}`);
      const resolved = new URL(value, base);
      let resolvedPath = resolved.pathname.replace(/^\//, "");
      if (!resolvedPath) {
        resolvedPath = "index.html";
      }
      if (resolvedPath.endsWith("/")) {
        resolvedPath = `${resolvedPath}index.html`;
      }

      if (HTML_EXT_RE.test(resolvedPath)) {
        const sourceFile = join(HISTORY_SOURCE_DIR, ...resolvedPath.split("/"));
        const mapped = HISTORY_SECTION_BY_SOURCE_PATH[resolvedPath];
        if (mapped || (isLocallyHostedHistorySourcePath(resolvedPath) && existsSync(sourceFile))) {
          return `${attr}="#/${historyRouteForSourcePath(resolvedPath)}"`;
        }
        return `${attr}="${sourceUrl(resolvedPath)}"`;
      }

      const sourceAsset = join(HISTORY_SOURCE_DIR, ...resolvedPath.split("/"));
      if (!existsSync(sourceAsset) || !statSync(sourceAsset).isFile()) {
        return `${attr}="${sourceUrl(resolvedPath)}"`;
      }

      assetPaths.add(resolvedPath);
      return `${attr}="./public/history/assets/${resolvedPath}"`;
    },
  );
}

function deriveTitle(html: string, fallback: string): string {
  const match = html.match(/<title>([\s\S]*?)<\/title>/i);
  if (!match) {
    return fallback;
  }
  const cleaned = match[1]
    .replace(/Anti-Grain Geometry\s*-\s*/i, "")
    .replace(/\s+/g, " ")
    .trim();
  return cleaned || fallback;
}

function ensureParentDir(path: string): void {
  const idx = path.lastIndexOf("/");
  if (idx <= 0) {
    return;
  }
  mkdirSync(path.slice(0, idx), { recursive: true });
}

function collectHtmlPaths(relativeDir: string): string[] {
  const root = join(HISTORY_SOURCE_DIR, ...relativeDir.split("/"));
  if (!existsSync(root)) {
    return [];
  }
  const results: string[] = [];
  const walk = (absDir: string, relDir: string): void => {
    for (const entry of readdirSync(absDir, { withFileTypes: true })) {
      const nextAbs = join(absDir, entry.name);
      const nextRel = relDir ? `${relDir}/${entry.name}` : entry.name;
      if (entry.isDirectory()) {
        walk(nextAbs, nextRel);
        continue;
      }
      if (HTML_EXT_RE.test(entry.name)) {
        results.push(`${relativeDir}/${nextRel}`.replaceAll("\\", "/"));
      }
    }
  };
  walk(root, "");
  return results;
}

function generateHistoryContent(): void {
  if (!existsSync(HISTORY_SOURCE_DIR)) {
    console.warn("  WARNING: cpp-references/agg-web not found - skipping AGG history generation");
    return;
  }

  rmSync(HISTORY_PUBLIC_DIR, { recursive: true, force: true });
  rmSync(join(DEMO_DIR, "public", "legacy"), { recursive: true, force: true });
  mkdirSync(HISTORY_FRAGMENTS_DIR, { recursive: true });
  mkdirSync(HISTORY_ASSETS_DIR, { recursive: true });

  const generatedAt = new Date().toISOString();
  const assetPaths = new Set<string>(["agg_logo.gif", "agg_title.jpg", "agg_button.gif", "download.gif", "link.gif"]);
  const entries: HistoryIndexEntry[] = [];
  const sourcePathsToGenerate = new Set<string>(HISTORY_SECTIONS.map((section) => section.sourcePath));
  for (const path of collectHtmlPaths("research")) {
    sourcePathsToGenerate.add(path);
  }
  for (const path of collectHtmlPaths("tips")) {
    sourcePathsToGenerate.add(path);
  }

  for (const sourcePath of sourcePathsToGenerate) {
    const section = HISTORY_SECTION_BY_SOURCE_PATH[sourcePath];
    const route = historyRouteForSourcePath(sourcePath);
    const sourceFile = join(HISTORY_SOURCE_DIR, ...sourcePath.split("/"));
    if (!existsSync(sourceFile)) {
      console.warn(`  WARNING: Missing history source page ${sourcePath}`);
      continue;
    }

    const sourceHtml = readFileSync(sourceFile, "utf8");
    const titleFallback = section?.title ?? sourcePath.replace(/^.*\//, "");
    const title = deriveTitle(sourceHtml, titleFallback);
    const stripped = stripLegacyChrome(sourceHtml);
    const rewritten = rewriteLinksAndCollectAssets(stripped, sourcePath, assetPaths);
    const fragmentFileName = `${route.replaceAll("/", "_")}.html`;
    const fragmentFile = join(HISTORY_FRAGMENTS_DIR, fragmentFileName);
    writeFileSync(fragmentFile, rewritten, "utf8");

    entries.push({
      route,
      sourcePath,
      sourceUrl: sourceUrl(sourcePath),
      title,
      contentPath: `./public/history/fragments/${fragmentFileName}`,
      generatedAt,
    });
  }

  for (const relative of assetPaths) {
    const sourceFile = join(HISTORY_SOURCE_DIR, ...relative.split("/"));
    if (!existsSync(sourceFile)) {
      continue;
    }
    const sourceMeta = lstatSync(sourceFile);
    if (!sourceMeta.isFile() || sourceMeta.isSymbolicLink()) {
      continue;
    }
    const target = slash(join(HISTORY_ASSETS_DIR, ...relative.split("/")));
    ensureParentDir(target);
    try {
      cpSync(sourceFile, target);
    } catch {
      // Skip problematic historical files rather than failing the whole site build.
      continue;
    }
  }

  writeFileSync(
    join(HISTORY_PUBLIC_DIR, "content-index.json"),
    JSON.stringify({ generatedAt, entries }, null, 2),
    "utf8",
  );
  console.log(`  Generated AGG history content for ${entries.length} sections`);
}

generateHistoryContent();

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

// Copy public/history/ (generated AGG history content)
const historyDir = join(DEMO_DIR, "public", "history");
if (existsSync(historyDir)) {
  cpSync(historyDir, join(DIST_DIR, "public", "history"), { recursive: true });
  console.log("  Copied public/history/ (history content)");
} else {
  console.warn("  WARNING: public/history/ not found");
}

console.log(`\nBuild complete: site assembled in dist/`);
