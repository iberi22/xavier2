# GitHub Pages Plan

## Current State

- `docs/site/` contains the Astro + Starlight docs site source
- `.github/workflows/docs.yml` builds and deploys the site
- `docs/` remains the canonical umbrella directory for all documentation assets

## Deployment Model

The site is published through GitHub Pages using GitHub Actions.

### Workflow Scope

- rebuild on changes under `docs/site/**`
- cache dependencies from `docs/site/package-lock.json`
- build from `docs/site`
- publish `docs/site/dist`

## Site Structure

```text
docs/site/
├── public/
├── src/
│   ├── content/docs/
│   │   ├── architecture/
│   │   ├── guides/
│   │   ├── modules/
│   │   ├── reference/
│   │   └── testing/
│   └── styles/
├── astro.config.mjs
├── package.json
└── tsconfig.json
```

## Local Workflow

```bash
cd docs/site
npm ci
npm run dev
npm run build
```

## Publishing Checklist

- keep all public docs content in English
- keep navigation aligned with Diataxis categories
- avoid duplicating agent-only material in the public docs site
- update the root `README.md` when site location or publishing flow changes
