import { useCallback, useContext, useEffect, useMemo, useState } from 'react';
import { CommandPaletteContext } from '@/contexts/CommandPaletteContext';
import type { CommandItem } from '@/contexts/CommandPaletteContext';
import { COMMAND_PALETTE } from '@/constants';

const fuzzyMatch = (text: string, query: string): boolean => {
  const lower = text.toLowerCase();
  const q = query.toLowerCase();
  let qi = 0;
  for (let i = 0; i < lower.length && qi < q.length; i++) {
    if (lower[i] === q[qi]) qi++;
  }
  return qi === q.length;
};

const useCommandPalette = () => {
  const ctx = useContext(CommandPaletteContext);
  if (!ctx) throw new Error('useCommandPalette must be used within CommandPaletteProvider');

  const { open, commands, recentIds, openPalette, closePalette, togglePalette, addRecent } = ctx;
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);

  // Global Cmd+K / Ctrl+K listener
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        togglePalette();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [togglePalette]);

  // Wrap openPalette to also reset state
  const openWithReset = useCallback(() => {
    setQuery('');
    setSelectedIndex(0);
    openPalette();
  }, [openPalette]);

  const filteredCommands = useMemo(() => {
    if (!query) {
      const recentSet = new Set(recentIds);
      const recent = recentIds
        .map((id) => commands.find((c) => c.id === id))
        .filter((c): c is CommandItem => c !== undefined)
        .map((c) => ({ ...c, group: 'recent' as const }));
      const rest = commands.filter((c) => !recentSet.has(c.id));
      return [...recent, ...rest].slice(0, COMMAND_PALETTE.MAX_RESULTS);
    }

    return commands
      .filter((c) => {
        if (fuzzyMatch(c.label, query)) return true;
        if (c.description && fuzzyMatch(c.description, query)) return true;
        if (c.keywords?.some((k) => fuzzyMatch(k, query))) return true;
        return false;
      })
      .slice(0, COMMAND_PALETTE.MAX_RESULTS);
  }, [query, commands, recentIds]);

  // Clamp selection via derived state
  const clampedIndex = selectedIndex >= filteredCommands.length
    ? Math.max(0, filteredCommands.length - 1)
    : selectedIndex;

  const executeSelected = useCallback(() => {
    const command = filteredCommands[clampedIndex];
    if (command) {
      addRecent(command.id);
      closePalette();
      command.action();
    }
  }, [filteredCommands, clampedIndex, addRecent, closePalette]);

  const moveSelection = useCallback(
    (direction: 'up' | 'down') => {
      setSelectedIndex((prev) => {
        if (direction === 'up') return prev > 0 ? prev - 1 : filteredCommands.length - 1;
        return prev < filteredCommands.length - 1 ? prev + 1 : 0;
      });
    },
    [filteredCommands.length],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault();
          moveSelection('down');
          break;
        case 'ArrowUp':
          e.preventDefault();
          moveSelection('up');
          break;
        case 'Enter':
          e.preventDefault();
          executeSelected();
          break;
        case 'Escape':
          e.preventDefault();
          closePalette();
          break;
      }
    },
    [moveSelection, executeSelected, closePalette],
  );

  return {
    open,
    query,
    setQuery,
    selectedIndex: clampedIndex,
    filteredCommands,
    handleKeyDown,
    executeSelected,
    openPalette: openWithReset,
    closePalette,
  };
};

export { useCommandPalette };
