import type {SortDirection} from './types'

/**
 * Generic sort comparator function
 */
function sortData<T>(
  data: T[],
  sortColumn: string | null,
  sortDirection: SortDirection,
  sortFn?: (a: T, b: T) => number,
): T[] {
  if (!sortColumn) return data

  const sorted = [...data].sort((a, b) => {
    // Use custom sort function if provided
    if (sortFn) {
      return sortFn(a, b)
    }

    // Default sort by column key
    const aVal = (a as Record<string, unknown>)[sortColumn]
    const bVal = (b as Record<string, unknown>)[sortColumn]

    // Handle null/undefined
    if (aVal === null || aVal === undefined) return 1
    if (bVal === null || bVal === undefined) return -1

    // String comparison
    if (typeof aVal === 'string' && typeof bVal === 'string') {
      return aVal.localeCompare(bVal, undefined, {numeric: true, sensitivity: 'base'})
    }

    // Number comparison
    if (typeof aVal === 'number' && typeof bVal === 'number') {
      return aVal - bVal
    }

    // Boolean comparison
    if (typeof aVal === 'boolean' && typeof bVal === 'boolean') {
      return aVal === bVal ? 0 : aVal ? -1 : 1
    }

    // Fallback: convert to string (handle objects)
    const aStr =
      typeof aVal === 'object'
        ? JSON.stringify(aVal)
        : String(aVal as string | number | boolean | bigint | symbol)
    const bStr =
      typeof bVal === 'object'
        ? JSON.stringify(bVal)
        : String(bVal as string | number | boolean | bigint | symbol)
    return aStr.localeCompare(bStr)
  })

  return sortDirection === 'desc' ? sorted.reverse() : sorted
}

/**
 * Filter data based on search query across specified fields
 */
function filterData<T>(
  data: T[],
  searchQuery: string,
  searchFields?: (keyof T)[],
): T[] {
  if (!searchQuery.trim()) return data

  const query = searchQuery.trim().toLowerCase()

  return data.filter((row) => {
    // If no search fields specified, search all string fields
    const fieldsToSearch = searchFields ?? (Object.keys(row as object) as (keyof T)[])

    return fieldsToSearch.some((field) => {
      const value = row[field]
      if (value === null || value === undefined) return false
      // Handle objects by converting to JSON
      const stringValue =
        typeof value === 'object' ? JSON.stringify(value) : String(value)
      return stringValue.toLowerCase().includes(query)
    })
  })
}

/**
 * Paginate data
 */
function paginateData<T>(data: T[], page: number, pageSize: number): T[] {
  const startIndex = page * pageSize
  const endIndex = startIndex + pageSize
  return data.slice(startIndex, endIndex)
}

/**
 * Get density-specific padding
 */
function getDensityPadding(density: 'compact' | 'normal' | 'comfortable'): string {
  switch (density) {
    case 'compact':
      return '4px 8px'
    case 'comfortable':
      return '16px'
    default:
      return '8px 12px'
  }
}

/**
 * Export data to CSV
 */
function exportToCSV<T>(
  data: T[],
  columns: {key: string; header: string}[],
  filename: string,
): void {
  // Create CSV header
  const header = columns.map((col) => col.header).join(',')

  // Create CSV rows
  const rows = data.map((row) =>
    columns
      .map((col) => {
        const value = (row as Record<string, unknown>)[col.key]
        // Escape quotes and wrap in quotes if contains comma
        let stringValue: string
        if (value === null || value === undefined) {
          stringValue = ''
        } else if (typeof value === 'object') {
          stringValue = JSON.stringify(value)
        } else {
          // value is a primitive type
          stringValue = String(value as string | number | boolean | bigint | symbol)
        }
        const escaped = stringValue.replace(/"/g, '""')
        return stringValue.includes(',') || stringValue.includes('"')
          ? `"${escaped}"`
          : escaped
      })
      .join(','),
  )

  // Combine header and rows
  const csv = [header, ...rows].join('\n')

  // Create blob and download
  const blob = new Blob([csv], {type: 'text/csv;charset=utf-8;'})
  const link = document.createElement('a')
  const url = URL.createObjectURL(blob)
  link.setAttribute('href', url)
  link.setAttribute('download', filename)
  link.style.visibility = 'hidden'
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
}

/**
 * Export data to JSON
 */
function exportToJSON<T>(data: T[], filename: string): void {
  const json = JSON.stringify(data, null, 2)
  const blob = new Blob([json], {type: 'application/json'})
  const link = document.createElement('a')
  const url = URL.createObjectURL(blob)
  link.setAttribute('href', url)
  link.setAttribute('download', filename)
  link.style.visibility = 'hidden'
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
}

export {sortData, filterData, paginateData, getDensityPadding, exportToCSV, exportToJSON}
