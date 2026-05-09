// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
  site: 'https://southwest-ai-labs.github.io/xavier/',
  integrations: [
    starlight({
      title: 'Xavier',
      description: 'Cognitive Memory for AI Swarms',
      pagefind: false,
      sidebar: [
        {
          label: 'Getting Started',
          items: [
            { label: 'Introduction', link: '/guides/intro/' },
            { label: 'Installation', link: '/guides/installation/' },
            { label: 'Quick Start', link: '/guides/quick-start/' },
          ],
        },
        {
          label: 'Architecture',
          autogenerate: { directory: 'architecture' },
        },
        {
          label: 'Modules',
          autogenerate: { directory: 'modules' },
        },
        {
          label: 'API Reference',
          autogenerate: { directory: 'reference' },
        },
        {
          label: 'Testing',
          autogenerate: { directory: 'testing' },
        },
      ],
      customCss: [
        './src/styles/custom.css',
      ],
    }),
  ],
});
