import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: ["Geist", "Inter", "system-ui", "sans-serif"],
        mono: ["Geist Mono", "SF Mono", "Menlo", "monospace"],
      },
      colors: {
        bg: {
          DEFAULT: "#000",
          1: "#0a0a0a",
          2: "#111",
          hover: "#191919",
        },
        border: {
          DEFAULT: "#222",
          light: "#333",
        },
        text: {
          DEFAULT: "#ededed",
          2: "#a1a1a1",
          3: "#666",
          4: "#444",
        },
        status: {
          green: "#0cce6b",
          amber: "#f5a623",
          red: "#ee0000",
        },
        accent: {
          blue: "#0070f3",
          purple: "#7928ca",
          cyan: "#50e3c2",
          pink: "#f81ce5",
        },
      },
    },
  },
  plugins: [],
};

export default config;
