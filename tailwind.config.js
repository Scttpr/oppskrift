/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./templates/**/*.html",
    "./src/**/*.rs",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // Instance theming via CSS custom properties
        primary: {
          50: 'var(--color-primary-50, #fef2f2)',
          100: 'var(--color-primary-100, #fee2e2)',
          200: 'var(--color-primary-200, #fecaca)',
          300: 'var(--color-primary-300, #fca5a5)',
          400: 'var(--color-primary-400, #f87171)',
          500: 'var(--color-primary-500, #ef4444)',
          600: 'var(--color-primary-600, #dc2626)',
          700: 'var(--color-primary-700, #b91c1c)',
          800: 'var(--color-primary-800, #991b1b)',
          900: 'var(--color-primary-900, #7f1d1d)',
          950: 'var(--color-primary-950, #450a0a)',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
      },
    },
  },
  plugins: [],
}
