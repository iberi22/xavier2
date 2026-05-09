# Xavier Docs Site

Published documentation site source for Xavier.

## Purpose

- Build the public documentation site from content under `src/content/docs/`
- Publish through the GitHub Pages workflow in `.github/workflows/docs.yml`
- Keep site-specific tooling isolated under `docs/site/`
- Keep public docs aligned with the current verified runtime, not with aspirational architecture alone

## Commands

```bash
npm ci
npm run dev
npm run build
npm run preview
```

## Content

- `src/content/docs/architecture/`
- `src/content/docs/guides/`
- `src/content/docs/modules/`
- `src/content/docs/reference/`
- `src/content/docs/testing/`
