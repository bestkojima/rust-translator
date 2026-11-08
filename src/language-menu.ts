export type LanguageMenu = 'source' | 'target' | null;

export function toggleLanguageMenu(current: LanguageMenu, requested: Exclude<LanguageMenu, null>): LanguageMenu {
  return current === requested ? null : requested;
}

// Escape first dismisses the open menu; another Escape dismisses the window.
export function escapeLanguageMenu(current: LanguageMenu): { menu: null; closeWindow: boolean } {
  return { menu: null, closeWindow: current === null };
}

export function moveLanguageOption(current: number, count: number, key: string): number {
  if (count <= 0) return 0;
  if (key === 'Home') return 0;
  if (key === 'End') return count - 1;
  if (key === 'ArrowDown') return (current + 1) % count;
  if (key === 'ArrowUp') return (current - 1 + count) % count;
  return current;
}

export function languageChanged(current: string, next: string): boolean { return current !== next; }
