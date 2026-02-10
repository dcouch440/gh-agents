import { createContext, useCallback, useMemo, useState, type ReactNode } from 'react';
import { LS_RECENT_COMMANDS, COMMAND_PALETTE } from '@/constants';
import { Collections } from '@/utils/collections';

type CommandItem = {
  id: string;
  label: string;
  description?: string;
  icon?: ReactNode;
  shortcut?: string;
  group: 'navigation' | 'actions' | 'recent';
  action: () => void;
  keywords?: string[];
};

type CommandPaletteState = {
  open: boolean;
  commands: CommandItem[];
  recentIds: string[];
  openPalette: () => void;
  closePalette: () => void;
  togglePalette: () => void;
  registerCommands: (commands: CommandItem[]) => void;
  unregisterCommands: (ids: string[]) => void;
  addRecent: (id: string) => void;
};

const CommandPaletteContext = createContext<CommandPaletteState | null>(null);

const loadRecentIds = (): string[] => {
  try {
    const stored = localStorage.getItem(LS_RECENT_COMMANDS);
    if (!stored) return [];
    const parsed: unknown = JSON.parse(stored);
    if (Array.isArray(parsed)) return parsed.filter((v): v is string => typeof v === 'string');
    return [];
  } catch {
    return [];
  }
};

const saveRecentIds = (ids: string[]) => {
  localStorage.setItem(LS_RECENT_COMMANDS, JSON.stringify(ids));
};

function CommandPaletteProvider({ children }: { children: ReactNode }) {
  const [open, setOpen] = useState(false);
  const [commands, setCommands] = useState<CommandItem[]>([]);
  const [recentIds, setRecentIds] = useState<string[]>(loadRecentIds);

  const openPalette = useCallback(() => setOpen(true), []);
  const closePalette = useCallback(() => setOpen(false), []);
  const togglePalette = useCallback(() => setOpen((v) => !v), []);

  const registerCommands = useCallback((newCommands: CommandItem[]) => {
    setCommands((prev) => {
      const ids = Collections.toSetBy(newCommands, (c) => c.id);
      const filtered = prev.filter((c) => !ids.has(c.id));
      return [...filtered, ...newCommands];
    });
  }, []);

  const unregisterCommands = useCallback((ids: string[]) => {
    const idSet = new Set(ids);
    setCommands((prev) => prev.filter((c) => !idSet.has(c.id)));
  }, []);

  const addRecent = useCallback((id: string) => {
    setRecentIds((prev) => {
      const next = [id, ...prev.filter((r) => r !== id)].slice(0, COMMAND_PALETTE.MAX_RECENT);
      saveRecentIds(next);
      return next;
    });
  }, []);

  const value = useMemo(
    () => ({ open, commands, recentIds, openPalette, closePalette, togglePalette, registerCommands, unregisterCommands, addRecent }),
    [open, commands, recentIds, openPalette, closePalette, togglePalette, registerCommands, unregisterCommands, addRecent],
  );

  return (
    <CommandPaletteContext.Provider value={value}>
      {children}
    </CommandPaletteContext.Provider>
  );
}

export { CommandPaletteContext, CommandPaletteProvider };
export type { CommandItem, CommandPaletteState };
