import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';
import node from '@astrojs/node';

export default defineConfig({
  integrations: [svelte()],
  output: 'static',
  adapter: node({
    mode: 'standalone'
  }),
  build: {
    format: 'file'
  },
  vite: {
    ssr: {
      noExternal: ['flexsearch']
    }
  }
});
