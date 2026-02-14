import { useState, useMemo, type ReactNode } from 'react'
import Box from '@mui/material/Box'
import { SearchInput, AccentBarRow, EmptyState, LoadingSpinner } from '@/components/primitives'
import { Collections } from '@/utils/collections'

type BrowserPanelRow = {
  primary: string
  secondary: string
}

type BrowserPanelProps<T extends { id: string }> = {
  items: readonly T[]
  loading: boolean
  searchPlaceholder: string
  emptyIcon: ReactNode
  emptyLabel: string
  barColor: string
  toRow: (item: T) => BrowserPanelRow
  matchesQuery: (item: T, query: string) => boolean
  isHighlighted: (item: T) => boolean
  onItemClick: ((itemId: string) => void) | null
}

function BrowserPanel<T extends { id: string }>({
  items,
  loading,
  searchPlaceholder,
  emptyIcon,
  emptyLabel,
  barColor,
  toRow,
  matchesQuery,
  isHighlighted,
  onItemClick,
}: BrowserPanelProps<T>) {
  const [query, setQuery] = useState('')

  const filtered = useMemo(() => {
    if (!query) return items
    return Collections.filterMap(items as T[], (item) => (matchesQuery(item, query) ? item : null))
  }, [items, query, matchesQuery])

  return (
    <Box>
      <Box sx={{ px: 1.5, py: 1 }}>
        <SearchInput value={query} onChange={setQuery} placeholder={searchPlaceholder} />
      </Box>

      {loading ? <LoadingSpinner label={`Loading ${emptyLabel}...`} /> : null}

      {!loading && filtered.length === 0 ? (
        <EmptyState icon={emptyIcon} message={query ? `No ${emptyLabel} matching "${query}"` : `No ${emptyLabel} found`} />
      ) : null}

      {filtered.map((item) => {
        const row = toRow(item)
        return (
          <AccentBarRow
            key={item.id}
            barColor={barColor}
            primary={row.primary}
            secondary={row.secondary}
            highlight={isHighlighted(item)}
            onClick={onItemClick ? () => onItemClick(item.id) : null}
          />
        )
      })}
    </Box>
  )
}

export { BrowserPanel }
export type { BrowserPanelProps, BrowserPanelRow }
