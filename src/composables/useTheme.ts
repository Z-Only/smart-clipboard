import { ref, watch, onMounted, onUnmounted } from "vue";

export type AppearanceMode = "system" | "light" | "dark";
export type ThemeColor = "zinc" | "blue" | "green" | "rose" | "orange" | "violet";

const APPEARANCE_KEY = "smart-clipboard-appearance";
const THEME_KEY = "smart-clipboard-theme";

const appearance = ref<AppearanceMode>(
  (localStorage.getItem(APPEARANCE_KEY) as AppearanceMode) || "system"
);
const themeColor = ref<ThemeColor>(
  (localStorage.getItem(THEME_KEY) as ThemeColor) || "zinc"
);

let mediaQuery: MediaQueryList | null = null;

function getSystemDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

function applyAppearance() {
  const isDark =
    appearance.value === "dark" ||
    (appearance.value === "system" && getSystemDark());

  document.documentElement.classList.toggle("dark", isDark);
}

function applyThemeColor() {
  const root = document.documentElement;
  // Remove all theme classes
  root.classList.remove(
    "theme-zinc",
    "theme-blue",
    "theme-green",
    "theme-rose",
    "theme-orange",
    "theme-violet"
  );
  // Add current theme class (zinc is default, no class needed but we add for consistency)
  root.classList.add(`theme-${themeColor.value}`);
}

function onSystemThemeChange() {
  if (appearance.value === "system") {
    applyAppearance();
  }
}

export function useTheme() {
  onMounted(() => {
    applyAppearance();
    applyThemeColor();

    mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    mediaQuery.addEventListener("change", onSystemThemeChange);
  });

  onUnmounted(() => {
    mediaQuery?.removeEventListener("change", onSystemThemeChange);
  });

  watch(appearance, (val) => {
    localStorage.setItem(APPEARANCE_KEY, val);
    applyAppearance();
  });

  watch(themeColor, (val) => {
    localStorage.setItem(THEME_KEY, val);
    applyThemeColor();
  });

  function setAppearance(mode: AppearanceMode) {
    appearance.value = mode;
  }

  function setThemeColor(color: ThemeColor) {
    themeColor.value = color;
  }

  return {
    appearance,
    themeColor,
    setAppearance,
    setThemeColor,
  };
}
