import { createTheme } from "@openuidev/react-ui";

export const designTokens = {
  color: {
    bg: "#F6F3EA",
    surface: "#FFFDF8",
    surface2: "#EDE7D8",
    text: "#111111",
    border: "#000000",
    accent: "#F2F230",
    accentStrong: "#D8D81E",
    danger: "#FF5C5C",
    info: "#3B82F6",
    success: "#19C37D",
    muted: "#5D584B",
  },
  shadow: {
    card: "6px 6px 0 #000000",
    button: "4px 4px 0 #000000",
    focus: "0 0 0 3px #000000, 0 0 0 6px #F2F230",
  },
  radius: {
    sm: "8px",
    md: "10px",
  },
  motion: {
    hover: "translate(-2px, -2px)",
    press: "translate(0, 0)",
  },
  font: {
    ui: '"Lexend", "Inter Tight", "Segoe UI", sans-serif',
    mono: '"IBM Plex Mono", "Cascadia Code", monospace',
  },
} as const;

export const theme = createTheme({
  background: designTokens.color.bg,
  foreground: designTokens.color.text,
  elevated: designTokens.color.surface,
  elevatedStrong: designTokens.color.surface2,
  textNeutralPrimary: designTokens.color.text,
  textNeutralSecondary: designTokens.color.muted,
  borderDefault: designTokens.color.border,
  borderInteractive: designTokens.color.border,
  borderAccent: designTokens.color.accentStrong,
  interactiveAccentDefault: designTokens.color.accentStrong,
  interactiveAccentHover: designTokens.color.accent,
  interactiveAccentPressed: designTokens.color.accentStrong,
  interactiveDestructiveDefault: designTokens.color.danger,
  interactiveDestructiveHover: designTokens.color.danger,
  interactiveDestructivePressed: designTokens.color.danger,
  fontBody: designTokens.font.ui,
  fontHeading: designTokens.font.ui,
  fontCode: designTokens.font.mono,
  fontLabel: designTokens.font.mono,
  radiusM: designTokens.radius.sm,
  radiusL: designTokens.radius.md,
  shadowM: designTokens.shadow.button,
  shadowL: designTokens.shadow.card,
});
