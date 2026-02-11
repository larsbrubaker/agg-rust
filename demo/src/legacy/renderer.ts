import {
  HISTORY_SECTIONS,
  HISTORY_SECTION_BY_ROUTE,
  historySourcePathForRoute,
  isLocallyHostedHistorySourcePath,
  type HistorySection,
} from './sections.ts';

type HistoryIndexEntry = {
  route: string;
  sourcePath: string;
  sourceUrl: string;
  title: string;
  contentPath: string;
  generatedAt: string;
};

type HistoryContentIndex = {
  generatedAt: string;
  entries: HistoryIndexEntry[];
};

let contentIndexPromise: Promise<HistoryContentIndex | null> | null = null;

function getSectionRoute(route: string): string {
  if (route === 'history') {
    return 'history/home';
  }
  if (route === 'legacy') return 'history/home';
  if (route.startsWith('legacy/')) return `history/${route.slice('legacy/'.length)}`;
  return route;
}

function sourceUrlForPath(path: string): string {
  return `https://agg.sourceforge.net/antigrain.com/${path}`;
}

async function loadContentIndex(): Promise<HistoryContentIndex | null> {
  if (!contentIndexPromise) {
    contentIndexPromise = fetch('./public/history/content-index.json')
      .then(async (response) => {
        if (!response.ok) {
          return null;
        }
        return (await response.json()) as HistoryContentIndex;
      })
      .catch(() => null);
  }
  return contentIndexPromise;
}

function indexByRoute(index: HistoryContentIndex | null): Record<string, HistoryIndexEntry> {
  if (!index) {
    return {};
  }
  return Object.fromEntries(index.entries.map((entry) => [entry.route, entry]));
}

function rustLinksHtml(section: HistorySection): string {
  if (!section.rustRoutes || section.rustRoutes.length === 0) {
    return '';
  }
  const links = section.rustRoutes
    .map((route) => `<a href="#/${route}" class="legacy-chip">${route}</a>`)
    .join('');
  return `
    <div class="legacy-rust-links">
      <h4>Related Rust Routes</h4>
      <div class="legacy-chip-row">${links}</div>
    </div>
  `;
}

function tributeBanner(section: HistorySection, sourceUrl: string, sourcePath: string): string {
  const archiveNote = section.archiveNote
    ? `<p class="legacy-note">${section.archiveNote}</p>`
    : '';
  const portUpdateNote = section.portUpdateNote
    ? `<p class="legacy-port-update">${section.portUpdateNote}</p>`
    : '';
  const currentLinks = section.currentLinks && section.currentLinks.length > 0
    ? section.currentLinks
        .map((link) => `<a href="${link.href}" target="_blank" rel="noreferrer">${link.label}</a>`)
        .join('')
    : '';
  const showOriginalLink = !isLocallyHostedHistorySourcePath(sourcePath);
  const originalLink = showOriginalLink
    ? `<a href="${sourceUrl}" target="_blank" rel="noreferrer">Original page on SourceForge mirror</a>`
    : `<span class="legacy-local-note">Original AGG content is hosted locally on this site.</span>`;
  return `
    <section class="legacy-banner">
      <h1>${section.title}</h1>
      <p>${section.description}</p>
      ${archiveNote}
      ${portUpdateNote}
      <div class="legacy-banner-links">
        ${originalLink}
        ${currentLinks}
      </div>
    </section>
  `;
}

function formatDate(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return iso;
  }
  return date.toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}

function livingUpdateHtml(sectionRoute: string, generatedAt?: string): string {
  if (sectionRoute !== 'history/about') {
    return '';
  }
  const dateLabel = formatDate(generatedAt ?? new Date().toISOString());

  return `
    <section class="legacy-living-update">
      <h2>Rust Port: Current Work</h2>
      <p class="legacy-current-date">Status date: <strong>${dateLabel}</strong></p>
      <p>
        The original AGG project remains the foundation, and this site now tracks the
        active Rust port as a living continuation of that work.
      </p>
      <h3>Current focus</h3>
      <ul>
        <li>Porting and validating AGG modules with behavior parity against the original C++ implementation.</li>
        <li>Maintaining and expanding interactive WebAssembly demos while preserving AGG visual fidelity.</li>
        <li>Keeping this History and Living Project section in sync with current Rust-port progress.</li>
        <li>Using Rust-native polygon workflows (including <a href="https://github.com/larsbrubaker/clipper2-rust" target="_blank" rel="noreferrer">clipper2-rust</a> patterns and the <a href="https://crates.io/crates/clipper2-rust" target="_blank" rel="noreferrer">clipper2-rust crate</a>); GPC is not part of this port.</li>
      </ul>
      <p>
        See the main demo index and project repository for the latest implementation status.
      </p>
    </section>
  `;
}

function contactPageHtml(sectionRoute: string): string {
  if (sectionRoute !== 'history/contact') {
    return '';
  }

  return `
    <section class="legacy-living-update legacy-contact-note">
      <h2>In Memory of Maxim Shemanarev</h2>
      <div class="legacy-contact-image-wrap">
        <img
          class="legacy-contact-image"
          src="./public/history/assets/mcseem/mcseem.jpg"
          alt="Maxim Shemanarev"
          loading="lazy"
        />
      </div>
      <p>
        Maxim Shemanarev, the original author of Anti-Grain Geometry (AGG), created one of the
        most influential software rendering libraries in modern graphics programming.
      </p>
      <p>
        This project is built with deep respect for his work and its long-lasting impact.
      </p>
      <p>
        Historical contact information from the original site is no longer valid.
      </p>
      <p>
        For questions about this Rust port, please contact
        <a href="mailto:larsbrubaker@gmail.com">larsbrubaker@gmail.com</a>.
      </p>
      <p>
        Learn more about AGG:
        <a href="https://en.wikipedia.org/wiki/Anti-Grain_Geometry" target="_blank" rel="noreferrer">Anti-Grain Geometry on Wikipedia</a>.
      </p>
    </section>
  `;
}

function mapDemoStemToRoute(stem: string, availableRoutes: Set<string>): string | null {
  if (availableRoutes.has(stem)) {
    return stem;
  }
  // gpc_test is intentionally replaced by the Rust boolean demo workflow.
  if (stem === 'gpc_test' && availableRoutes.has('scanline_boolean2')) {
    return 'scanline_boolean2';
  }
  if (stem === 'freetype_test' && availableRoutes.has('truetype_test')) {
    return 'truetype_test';
  }
  return null;
}

function rustDemoSourceUrl(route: string): string {
  return `https://github.com/larsbrubaker/agg-rust/blob/master/demo/src/demos/${route}.ts`;
}

function enhanceDemoTable(container: HTMLElement): void {
  const content = container.querySelector('.legacy-content') as HTMLElement | null;
  if (!content) {
    return;
  }

  const table = content.querySelector('table.tbl') as HTMLTableElement | null;
  if (!table) {
    return;
  }

  // Replace the legacy executable/download framing with Rust-port context.
  const oldIntroTables = Array.from(content.querySelectorAll('table')).filter((t) => t !== table);
  for (const t of oldIntroTables) {
    const hasDemoText = /demo examples|download|to be continued/i.test(t.textContent || '');
    if (hasDemoText) {
      t.remove();
    }
  }
  content.insertAdjacentHTML(
    'afterbegin',
    `
      <section class="legacy-living-update history-demo-rust-intro">
        <h2>Rust Demo Index</h2>
        <p>
          This section tracks the Rust/WebAssembly demos in this repository. Use
          <strong>Open Rust demo</strong> to launch the live demo on this site and
          <strong>View Rust source</strong> to open the TypeScript demo entrypoint in GitHub.
        </p>
        <p>
          The original C++ demo references are preserved as historical context, but this page is
          now focused on the active Rust port.
        </p>
      </section>
    `,
  );

  const availableRoutes = new Set(
    Array.from(document.querySelectorAll('.nav-link[data-route]'))
      .map((el) => (el as HTMLElement).dataset.route || '')
      .filter(Boolean),
  );

  // Remove "Executable" column from header.
  const headerRow = table.querySelector('tr');
  if (headerRow) {
    const ths = headerRow.querySelectorAll('th');
    if (ths.length >= 3) {
      ths[2].remove();
    }
  }

  const rows = Array.from(table.querySelectorAll('tr'));
  for (const row of rows) {
    const tds = row.querySelectorAll('td');
    if (tds.length >= 3) {
      tds[2].remove();
    }
    if (tds.length < 2) {
      continue;
    }

    const screenshotCell = tds[0];
    const descriptionCell = tds[1];
    const rowText = (descriptionCell.textContent || '').trim().toLowerCase();
    if (rowText.includes('all examples in one package')) {
      row.remove();
      continue;
    }
    const codeLink = descriptionCell.querySelector('code a') as HTMLAnchorElement | null;
    if (!codeLink) {
      continue;
    }

    const stemMatch = (codeLink.textContent || '').match(/([a-z0-9_]+)\.cpp/i);
    if (!stemMatch) {
      continue;
    }
    const stem = stemMatch[1];
    const route = mapDemoStemToRoute(stem, availableRoutes);
    const sourceLabel = descriptionCell.querySelector('code') as HTMLElement | null;

    const existingStatus = screenshotCell.querySelector('.history-demo-status');
    if (existingStatus) {
      existingStatus.remove();
    }
    const status = document.createElement('div');
    status.className = 'history-demo-status';

    const existingLink = descriptionCell.querySelector('.history-demo-rust-link');
    if (existingLink) {
      existingLink.remove();
    }
    const rustLink = document.createElement('div');
    rustLink.className = 'history-demo-rust-link';

    if (route) {
      const sourceUrl = rustDemoSourceUrl(route);
      codeLink.setAttribute('href', sourceUrl);
      codeLink.setAttribute('target', '_blank');
      codeLink.setAttribute('rel', 'noreferrer');
      codeLink.textContent = `${route}.ts`;
      if (sourceLabel) {
        sourceLabel.insertAdjacentHTML(
          'beforeend',
          ` <span class="history-demo-source-tag">Rust</span>`,
        );
      }
      status.innerHTML = `<a href="#/${route}">Open Rust Demo</a>`;
      rustLink.innerHTML = `<a href="#/${route}">Open Rust demo</a> &middot; <a href="${sourceUrl}" target="_blank" rel="noreferrer">View Rust source</a>`;
    } else {
      codeLink.setAttribute('href', sourceUrlForPath(`demo/${stem}.cpp.html`));
      codeLink.setAttribute('target', '_blank');
      codeLink.setAttribute('rel', 'noreferrer');
      status.classList.add('history-demo-soon');
      status.textContent = 'Coming soon';
      rustLink.innerHTML = 'Rust demo coming soon';
      rustLink.classList.add('history-demo-soon');
    }

    screenshotCell.appendChild(status);
    descriptionCell.appendChild(rustLink);
  }
}

function renderLanding(container: HTMLElement): void {
  const cards = HISTORY_SECTIONS.map((section) => {
    return `
      <a href="#/${section.route}" class="legacy-card">
        <h3>${section.title}</h3>
        <p>${section.description}</p>
      </a>
    `;
  }).join('');

  container.innerHTML = `
    <div class="legacy-page home-page">
      <section class="legacy-tribute-intro">
        <h1>AGG History and Living Project</h1>
        <p>
          This section preserves and modernizes the original Anti-Grain Geometry website by
          <strong>Maxim Shemanarev</strong> while documenting ongoing Rust-port progress.
        </p>
        <p>
          Think of it as a living history: original context, modern presentation, and direct
          links to active Rust demos and current implementation work.
        </p>
      </section>
      <section class="legacy-grid">${cards}</section>
    </div>
  `;
}

export async function renderHistoryRoute(container: HTMLElement, route: string): Promise<void> {
  if (route === 'history' || route === 'legacy') {
    renderLanding(container);
    return;
  }

  const sectionRoute = getSectionRoute(route);
  container.innerHTML = `
    <div class="legacy-page home-page">
      <p class="legacy-loading">Loading history content...</p>
    </div>
  `;

  const contentIndex = await loadContentIndex();
  const entriesByRoute = indexByRoute(contentIndex);
  const entry = entriesByRoute[sectionRoute];
  const sourcePath = entry?.sourcePath ?? historySourcePathForRoute(sectionRoute);
  if (!sourcePath) {
    container.innerHTML = `
      <div class="legacy-page home-page">
        <h2>History page not found</h2>
        <p>Unknown route: ${route}</p>
      </div>
    `;
    return;
  }
  const section = HISTORY_SECTION_BY_ROUTE[sectionRoute] ?? {
    route: sectionRoute,
    title: entry?.title ?? 'AGG Article',
    sourcePath,
    description: isLocallyHostedHistorySourcePath(sourcePath)
      ? 'Historical AGG article hosted locally in the project history section.'
      : 'Historical AGG page.',
  };
  const sourceUrl = entry?.sourceUrl ?? sourceUrlForPath(sourcePath);

  if (!entry) {
    container.innerHTML = `
      <div class="legacy-page home-page">
        ${tributeBanner(section, sourceUrl, sourcePath)}
        ${livingUpdateHtml(sectionRoute, contentIndex?.generatedAt)}
        ${contactPageHtml(sectionRoute)}
        <div class="legacy-fallback">
          <p>
            Local generated content is not available yet for this page.
            Run the demo build to generate tribute fragments.
          </p>
          <p>
            <a href="${sourceUrl}" target="_blank" rel="noreferrer">Open original page</a>
          </p>
        </div>
        ${rustLinksHtml(section)}
      </div>
    `;
    return;
  }

  const response = await fetch(entry.contentPath);
  if (!response.ok) {
    container.innerHTML = `
      <div class="legacy-page home-page">
        ${tributeBanner(section, sourceUrl, sourcePath)}
        ${livingUpdateHtml(sectionRoute, contentIndex?.generatedAt)}
        ${contactPageHtml(sectionRoute)}
        <div class="legacy-fallback">
          <p>Generated content could not be loaded from <code>${entry.contentPath}</code>.</p>
          <p><a href="${sourceUrl}" target="_blank" rel="noreferrer">Open original page</a></p>
        </div>
        ${rustLinksHtml(section)}
      </div>
    `;
    return;
  }

  const fragment = await response.text();
  const content = sectionRoute === 'history/contact'
    ? ''
    : `<article class="legacy-content">${fragment}</article>`;
  container.innerHTML = `
    <div class="legacy-page home-page">
      ${tributeBanner(section, sourceUrl, sourcePath)}
      ${livingUpdateHtml(sectionRoute, entry.generatedAt)}
      ${contactPageHtml(sectionRoute)}
      ${content}
      ${rustLinksHtml(section)}
    </div>
  `;
  if (sectionRoute === 'history/demo') {
    enhanceDemoTable(container);
  }
}
