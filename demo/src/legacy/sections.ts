export type HistorySection = {
  route: string;
  title: string;
  sourcePath: string;
  description: string;
  rustRoutes?: string[];
  archiveNote?: string;
  portUpdateNote?: string;
  currentLinks?: Array<{ label: string; href: string }>;
};

export const HISTORY_SECTIONS: HistorySection[] = [
  {
    route: 'history/home',
    title: 'Main Page',
    sourcePath: 'index.html',
    description: 'Original Anti-Grain Geometry landing page and primary navigation.',
    rustRoutes: ['home'],
  },
  {
    route: 'history/about',
    title: 'About',
    sourcePath: 'about/index.html',
    description: 'Project philosophy, original AGG context, and the current Rust-port direction.',
    rustRoutes: ['home', 'lion', 'gradients'],
    portUpdateNote:
      'Rust-port update: references to the historical General Polygon Clipper are kept for context only. This project uses modern Rust approaches (including <a href="https://github.com/larsbrubaker/clipper2-rust" target="_blank" rel="noreferrer">clipper2-rust</a> workflows and the <a href="https://crates.io/crates/clipper2-rust" target="_blank" rel="noreferrer">clipper2-rust crate</a>) and does not ship GPC.',
  },
  {
    route: 'history/news',
    title: 'News',
    sourcePath: 'news/index.html',
    description: 'Historical release timeline and development updates.',
    portUpdateNote:
      'Rust-port update: legacy news entries may mention GPC as part of original AGG history; this port does not include GPC and follows Rust-native alternatives.',
  },
  {
    route: 'history/license',
    title: 'License',
    sourcePath: 'license/index.html',
    description: 'Original AGG licensing notes and historical license text.',
    archiveNote:
      'This section is preserved for historical context. For this Rust port, see repository licensing details.',
    portUpdateNote:
      'Rust-port update: the GPC component/license text is preserved historically, but GPC is not used in this Rust port. The Rust port uses <a href="https://github.com/larsbrubaker/clipper2-rust" target="_blank" rel="noreferrer">clipper2-rust</a>-based workflows and the <a href="https://crates.io/crates/clipper2-rust" target="_blank" rel="noreferrer">clipper2-rust crate</a> for modern polygon operations where needed.',
  },
  {
    route: 'history/download',
    title: 'Download',
    sourcePath: 'download/index.html',
    description: 'Historical package archives and legacy distribution notes.',
    archiveNote:
      'Download links are historical snapshots. Prefer the Rust repository and releases for current work.',
    currentLinks: [
      { label: 'Rust Port on GitHub', href: 'https://github.com/larsbrubaker/agg-rust' },
      { label: 'Crate on crates.io', href: 'https://crates.io/crates/agg-rust' },
      { label: 'API Docs on docs.rs', href: 'https://docs.rs/agg-rust' },
      { label: 'Rust Port Releases', href: 'https://github.com/larsbrubaker/agg-rust/releases' },
    ],
  },
  {
    route: 'history/screenshots',
    title: 'Screenshots',
    sourcePath: 'screenshots/index.html',
    description: 'Image gallery from the original AGG website.',
    rustRoutes: ['home'],
  },
  {
    route: 'history/demo',
    title: 'Demo',
    sourcePath: 'demo/index.html',
    description: 'Original demo index and platform-era screenshots.',
    rustRoutes: ['home', 'aa_demo', 'lion', 'compositing', 'image_resample'],
    portUpdateNote:
      'Rust-port update: demo references to GPC-based workflows are historical; this Rust port uses non-GPC paths aligned with current Rust tooling.',
  },
  {
    route: 'history/svg',
    title: 'SVG Viewer',
    sourcePath: 'svg/index.html',
    description: 'Legacy SVG viewer references and related materials.',
  },
  {
    route: 'history/docs',
    title: 'Documentation',
    sourcePath: 'doc/index.html',
    description: 'Core AGG documentation overview and doc entry points.',
    portUpdateNote:
      'Rust-port update: documentation links referencing `conv_gpc` are historical AGG references and are not active design choices for this Rust port.',
  },
  {
    route: 'history/tips',
    title: 'Tips and Tricks',
    sourcePath: 'tips/index.html',
    description: 'Practical techniques and usage notes from the original site.',
  },
  {
    route: 'history/research',
    title: 'Research and Articles',
    sourcePath: 'research/index.html',
    description: 'Technical research articles and deep dives by Maxim Shemanarev.',
  },
  {
    route: 'history/svn',
    title: 'SVN',
    sourcePath: 'svn/index.html',
    description: 'Legacy source control references from the original site.',
  },
  {
    route: 'history/sponsors',
    title: 'Sponsors',
    sourcePath: 'sponsors/index.html',
    description: 'Organizations that supported AGG development.',
  },
  {
    route: 'history/customers',
    title: 'Users and Customers',
    sourcePath: 'customers/index.html',
    description: 'Historical user and customer showcase.',
  },
  {
    route: 'history/links',
    title: 'Links and Friends',
    sourcePath: 'links/index.html',
    description: 'Related sites and references from the AGG ecosystem.',
    portUpdateNote:
      'Rust-port update: third-party GPC links are kept as historical references only; current polygon workflows use Rust-native alternatives.',
  },
  {
    route: 'history/contact',
    title: 'Contact',
    sourcePath: 'mcseem/index.html',
    description: 'Original author contact page and profile.',
  },
];

export const HISTORY_SECTION_BY_ROUTE: Record<string, HistorySection> = Object.fromEntries(
  HISTORY_SECTIONS.map((section) => [section.route, section]),
);

export const HISTORY_SECTION_BY_SOURCE_PATH: Record<string, HistorySection> = Object.fromEntries(
  HISTORY_SECTIONS.map((section) => [section.sourcePath, section]),
);

export function historyRouteForSourcePath(sourcePath: string): string {
  const section = HISTORY_SECTION_BY_SOURCE_PATH[sourcePath];
  if (section) {
    return section.route;
  }
  return `history/page/${sourcePath}`;
}

export function historySourcePathForRoute(route: string): string | null {
  const section = HISTORY_SECTION_BY_ROUTE[route];
  if (section) {
    return section.sourcePath;
  }
  if (route.startsWith('history/page/')) {
    return route.slice('history/page/'.length);
  }
  return null;
}

export function isLocallyHostedHistorySourcePath(sourcePath: string): boolean {
  return sourcePath.startsWith('research/') || sourcePath.startsWith('tips/');
}
