/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        primary: {
          DEFAULT: '#0f766e',
          50: '#ccfbf1',
          100: '#99f6e4',
          200: '#5eead4',
          300: '#2dd4bf',
          400: '#14b8a6',
          500: '#0f766e',
          600: '#0d9488',
          700: '#0f766e',
          800: '#115e59',
          900: '#134e4a',
        },
        accent: '#b45309',
        surface: {
          light: '#fffdf8',
          dark: '#1e293b',
        },
        background: {
          light: '#f4efe7',
          dark: '#0f172a',
        },
      },
    },
  },
  plugins: [],
};
