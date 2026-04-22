/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        surface: {
          900: '#081D3A',
          800: '#0D2447',
          700: '#12305C',
          600: '#1A3A6B',
          500: '#224A85',
        },
        accent: {
          600: '#B8941F',
          500: '#D4AF37',
          400: '#E5C96E',
          300: '#F0DF9E',
        },
        ivory: '#F2EAD7',
      },
      fontFamily: {
        sans: ['Lato', 'system-ui', 'sans-serif'],
        display: ['"Trajan Pro"', 'Cinzel', 'serif'],
      },
    },
  },
  plugins: [],
}
