const APPLE_PLATFORM = /Mac|iPhone|iPad|iPod/i;

export function primaryModifierLabel(): "⌘" | "Ctrl" {
  if (typeof navigator === "undefined") return "Ctrl";
  return APPLE_PLATFORM.test(navigator.platform || navigator.userAgent)
    ? "⌘"
    : "Ctrl";
}

export function shortcutLabel(key: string): string {
  const modifier = primaryModifierLabel();
  return modifier === "⌘" ? `${modifier}${key}` : `${modifier} ${key}`;
}
