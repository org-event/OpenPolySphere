# GitHub Pages

The public site is in [`site/`](../site/) and deployed via GitHub Actions (`.github/workflows/pages.yml`).

| URL | Page |
|-----|------|
| https://org-event.github.io/OpenPolySphere/ | English |
| https://org-event.github.io/OpenPolySphere/ru/ | Russian |

Use the English URL as the **project homepage** for [OpenSSF Best Practices](https://www.bestpractices.dev/en/projects/13385).

**One-time repo setting:** Settings → Pages → Build and deployment → Source: **GitHub Actions** (not “Deploy from branch /docs”).

Local preview: open `site/index.html` in a browser.

Markdown docs (`linux.md`, `windows.md`, ADRs, etc.) stay in `docs/` and are linked from the site but not served as Pages.
