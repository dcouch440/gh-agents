import { useRef, useEffect, useMemo, useCallback, useContext } from 'react';
import Box from '@mui/material/Box';
import InputBase from '@mui/material/InputBase';
import SearchOutlined from '@mui/icons-material/SearchOutlined';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import { useCommandPalette } from '@/hooks/useCommandPalette';
import { CommandPaletteContext } from '@/contexts/CommandPaletteContext';
import { CommandDialog } from './CommandDialog';
import { CommandItemRow } from './CommandItem';
import { CommandGroup } from './CommandGroup';

function CommandPalette() {
  const {
    open,
    query,
    setQuery,
    selectedIndex,
    filteredCommands,
    handleKeyDown,
    closePalette,
  } = useCommandPalette();

  const ctx = useContext(CommandPaletteContext);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Focus input when palette opens
  useEffect(() => {
    if (open) {
      const timeout = setTimeout(() => inputRef.current?.focus(), 50);
      return () => clearTimeout(timeout);
    }
  }, [open]);

  // Scroll selected item into view
  useEffect(() => {
    if (!listRef.current) return;
    const selected = listRef.current.querySelector('[aria-selected="true"]');
    if (selected) {
      selected.scrollIntoView({ block: 'nearest' });
    }
  }, [selectedIndex]);

  const handleItemSelect = useCallback(
    (commandId: string, action: () => void) => {
      ctx?.addRecent(commandId);
      closePalette();
      action();
    },
    [ctx, closePalette],
  );

  // Group commands by group property
  const grouped = useMemo(() => {
    const groups = new Map<string, typeof filteredCommands>();
    let globalIndex = 0;

    const groupLabels: Record<string, string> = {
      recent: 'Recent',
      navigation: 'Navigation',
      actions: 'Actions',
    };

    const result: Array<{ group: string; label: string; items: Array<{ command: typeof filteredCommands[0]; index: number }> }> = [];

    for (const cmd of filteredCommands) {
      const group = cmd.group;
      if (!groups.has(group)) {
        groups.set(group, []);
      }
      groups.get(group)!.push(cmd);
    }

    for (const [group, commands] of groups) {
      const items = commands.map((command) => ({
        command,
        index: globalIndex++,
      }));
      result.push({ group, label: groupLabels[group] ?? group, items });
    }

    return result;
  }, [filteredCommands]);

  return (
    <CommandDialog open={open} onClose={closePalette}>
      {/* Search input */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          px: 2,
          py: 1.5,
        }}
        onKeyDown={handleKeyDown}
      >
        <SearchOutlined sx={{ color: 'text.secondary', mr: 1.5, fontSize: '1.25rem' }} />
        <InputBase
          inputRef={inputRef}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Type a command or search..."
          fullWidth
          sx={{
            fontSize: '0.9375rem',
            '& .MuiInputBase-input': {
              py: 0.5,
            },
          }}
        />
        <Typography
          variant="caption"
          sx={{
            color: 'text.secondary',
            fontFamily: 'monospace',
            fontSize: '0.65rem',
            px: 0.75,
            py: 0.25,
            borderRadius: 0.5,
            border: 1,
            borderColor: 'divider',
            flexShrink: 0,
            ml: 1,
          }}
        >
          ESC
        </Typography>
      </Box>

      <Divider />

      {/* Results */}
      <Box
        ref={listRef}
        role="listbox"
        sx={{
          maxHeight: 320,
          overflowY: 'auto',
          py: 1,
        }}
      >
        {filteredCommands.length === 0 ? (
          <Typography
            variant="body2"
            sx={{ px: 2, py: 3, textAlign: 'center', color: 'text.secondary' }}
          >
            No results found
          </Typography>
        ) : (
          grouped.map(({ group, label, items }) => (
            <CommandGroup key={group} label={label}>
              {items.map(({ command, index }) => (
                <CommandItemRow
                  key={command.id}
                  icon={command.icon}
                  label={command.label}
                  description={command.description}
                  shortcut={command.shortcut}
                  selected={index === selectedIndex}
                  onSelect={() => handleItemSelect(command.id, command.action)}
                />
              ))}
            </CommandGroup>
          ))
        )}
      </Box>
    </CommandDialog>
  );
}

export { CommandPalette };
