/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'class',
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        bg: '#09090b',
        'bg-2': '#18181b',
        'bg-3': '#27272a',
        border: '#3f3f46',
        accent: '#6366f1',
        'accent-2': '#4f46e5',
        green: '#22c55e',
        yellow: '#eab308',
        red: '#ef4444',
        orange: '#f97316',
        purple: '#a855f7',
        blue: '#3b82f6',
        cyan: '#06b6d4',
      },
    },
  },
  plugins: [],
};
