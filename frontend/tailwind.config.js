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
          900: '#0f0f13',
          800: '#1a1a22',
          700: '#242430',
          600: '#2e2e3e',
        },
        accent: {
          500: '#7c6af7',
          400: '#9d8ffa',
        },
      },
    },
  },
  plugins: [],
}
