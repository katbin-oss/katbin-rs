/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
      "../api/templates/**/*.{html,js,tera}"
  ],
  theme: {
    extend: {
      extend: {
        colors: {
          amber: '#ff9800',
          'light-grey': '#212121',
        },
      },
    },
  },
  plugins: [],
}

