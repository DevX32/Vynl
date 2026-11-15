export type ThemeId = "vynl" | "espresso";

export interface ThemeColors {
  bg: string;
  bgRaise: string;
  surface: string;
  text: string;
  dim: string;
  faint: string;
  line: string;
  lineStrong: string;
  green: string;
  red: string;
  redBg: string;
  amber: string;
  vynl: string;
  scrollbar: string;
  scrollbarHover: string;
  placeholderGlow: string;
}

export interface Theme {
  id: ThemeId;
  name: string;
  colors: ThemeColors;
}

export const THEME_STORAGE_KEY = "vynl.theme";
export const DEFAULT_THEME: ThemeId = "vynl";

export const THEMES: readonly Theme[] = [
  {
    id: "vynl",
    name: "Vynl",
    colors: {
      bg: "#0e0e10",
      bgRaise: "#151519",
      surface: "#131317",
      text: "#f0f0ec",
      dim: "#a3a39b",
      faint: "#7a7a74",
      line: "rgba(240, 240, 236, 0.03)",
      lineStrong: "rgba(240, 240, 236, 0.07)",
      green: "#8fd694",
      red: "#ff6b61",
      redBg: "rgba(255, 107, 97, 0.5)",
      amber: "#d9a15c",
      vynl: "#14141c",
      scrollbar: "rgba(240, 240, 236, 0.12)",
      scrollbarHover: "rgba(240, 240, 236, 0.22)",
      placeholderGlow: "rgba(185, 167, 255, 0.08)",
    },
  },
  {
    id: "espresso",
    name: "Espresso",
    colors: {
      bg: "rgba(0, 0, 0, 0.84)",
      bgRaise: "rgba(18, 18, 18, 0.9)",
      surface: "rgba(9, 9, 9, 0.82)",
      text: "#cad3f5",
      dim: "#a5adcb",
      faint: "#8087a2",
      line: "rgba(202, 211, 245, 0.04)",
      lineStrong: "rgba(202, 211, 245, 0.09)",
      green: "#a6da95",
      red: "#ed8796",
      redBg: "rgba(237, 135, 150, 0.45)",
      amber: "#eed49f",
      vynl: "#050505",
      scrollbar: "rgba(202, 211, 245, 0.14)",
      scrollbarHover: "rgba(202, 211, 245, 0.24)",
      placeholderGlow: "rgba(198, 160, 246, 0.09)",
    },
  },
] as const;

const THEME_BY_ID = new Map<string, Theme>(
  THEMES.map((theme) => [theme.id, theme] as const),
);

const CSS_VARS: Record<keyof ThemeColors, string> = {
  bg: "--bg",
  bgRaise: "--bg-raise",
  surface: "--surface",
  text: "--text",
  dim: "--dim",
  faint: "--faint",
  line: "--line",
  lineStrong: "--line-strong",
  green: "--green",
  red: "--red",
  redBg: "--red-bg",
  amber: "--amber",
  vynl: "--vynl",
  scrollbar: "--scrollbar",
  scrollbarHover: "--scrollbar-hover",
  placeholderGlow: "--placeholder-glow",
};

function solidColor(color: string): string {
  const match = color.match(/^rgba?\(([^)]+)\)$/i);
  if (!match) return color;
  const [r, g, b] = match[1].split(",").map((part) => part.trim());
  return `rgb(${r}, ${g}, ${b})`;
}

export function isThemeId(value: string | null | undefined): value is ThemeId {
  return typeof value === "string" && THEME_BY_ID.has(value);
}

export function getTheme(id: string | null | undefined): Theme {
  return (isThemeId(id) ? THEME_BY_ID.get(id) : undefined) ?? THEMES[0];
}

export function applyTheme(
  id: string | null | undefined,
  persist = true,
): ThemeId {
  const theme = getTheme(id);
  const root = document.documentElement;
  root.dataset.theme = theme.id;
  root.style.colorScheme = "dark";

  for (const key of Object.keys(CSS_VARS) as (keyof ThemeColors)[]) {
    root.style.setProperty(CSS_VARS[key], theme.colors[key]);
  }

  root.style.setProperty("--bg-solid", solidColor(theme.colors.bg));
  root.style.setProperty("--bg-raise-solid", solidColor(theme.colors.bgRaise));
  root.style.setProperty("--surface-solid", solidColor(theme.colors.surface));

  root.style.setProperty(
    "--placeholder-gradient",
    `radial-gradient(circle at 50% 50%, ${theme.colors.placeholderGlow} 0 14%, transparent 30%), ` +
      `repeating-radial-gradient(circle at 50% 50%, var(--bg-raise) 0 2px, var(--bg) 2px 4px)`,
  );

  if (
    persist &&
    typeof localStorage !== "undefined" &&
    !window.location.hash.startsWith("#mini")
  ) {
    localStorage.setItem(THEME_STORAGE_KEY, theme.id);
  }

  return theme.id;
}

export function applyStoredTheme(): void {
  if (typeof localStorage === "undefined") return;
  applyTheme(localStorage.getItem(THEME_STORAGE_KEY), false);
}
