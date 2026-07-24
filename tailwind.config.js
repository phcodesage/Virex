/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class", '[data-theme="dark"]'],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          "SF Pro Text",
          "Inter",
          "system-ui",
          "sans-serif",
        ],
      },
      keyframes: {
        "pop-in": {
          "0%": { opacity: "0", transform: "scale(0.95) translateY(4px)" },
          "100%": { opacity: "1", transform: "scale(1) translateY(0)" },
        },
        "fade-out": {
          "0%": { opacity: "1", transform: "scale(1)" },
          "100%": { opacity: "0", transform: "scale(0.97)" },
        },
        shimmer: {
          "100%": { transform: "translateX(100%)" },
        },
      },
      animation: {
        "pop-in": "pop-in 140ms cubic-bezier(0.16, 1, 0.3, 1)",
        "fade-out": "fade-out 120ms ease-in forwards",
        shimmer: "shimmer 1.4s infinite",
      },
    },
  },
  plugins: [],
};
