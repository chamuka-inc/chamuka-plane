// JS-accessible design tokens — mirrors @theme in globals.css.
// Used by libraries that need raw color strings (xterm).

export const tokens = {
  bg: "#000",
  bg1: "#0a0a0a",
  bg2: "#111",
  bgHover: "#191919",
  border: "#222",
  borderLight: "#333",
  text: "#ededed",
  text2: "#a1a1a1",
  text3: "#666",
  text4: "#444",
  statusGreen: "#0cce6b",
  statusAmber: "#f5a623",
  statusRed: "#ee0000",
  accentBlue: "#0070f3",
  accentPurple: "#7928ca",
  accentCyan: "#50e3c2",
  accentPink: "#f81ce5",
  radiusSm: "4px",
  radiusMd: "6px",
  radiusLg: "8px",
  radiusXl: "12px",
} as const;
