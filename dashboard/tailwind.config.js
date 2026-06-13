/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        background: "#0A0C10",
        surface: "#161B22",
        accent: {
          blue: "#58A6FF",
          green: "#7EE787",
          orange: "#FFA657",
        }
      }
    },
  },
  plugins: [],
}
