import {describe, it, expect} from 'vitest'
import {renderHook, act} from '@testing-library/react'
import {useTableState} from './useTableState'
import type {TableColumn} from './types'

type TestRow = {
  id: string
  name: string
  age: number
  status: string
}

const mockData: TestRow[] = [
  {id: '1', name: 'Alice', age: 30, status: 'active'},
  {id: '2', name: 'Bob', age: 25, status: 'inactive'},
  {id: '3', name: 'Charlie', age: 35, status: 'active'},
  {id: '4', name: 'David', age: 28, status: 'active'},
  {id: '5', name: 'Eve', age: 32, status: 'inactive'},
]

const mockColumns: TableColumn<TestRow>[] = [
  {key: 'name', header: 'Name', sortable: true},
  {key: 'age', header: 'Age', sortable: true},
  {key: 'status', header: 'Status', sortable: true},
]

describe('useTableState', () => {
  describe('basic rendering', () => {
    it('returns all data when no features enabled', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enableSorting: false,
          enableSearch: false,
          enablePagination: false,
        }),
      )

      expect(result.current.displayedData).toEqual(mockData)
      expect(result.current.totalRows).toBe(5)
      expect(result.current.filteredRows).toBe(5)
    })
  })

  describe('sorting', () => {
    it('sorts data ascending when sort is clicked first time', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enableSorting: true,
        }),
      )

      act(() => {
        result.current.handleSort('name')
      })

      expect(result.current.sortColumn).toBe('name')
      expect(result.current.sortDirection).toBe('asc')
      expect(result.current.displayedData[0].name).toBe('Alice')
      expect(result.current.displayedData[4].name).toBe('Eve')
    })

    it('sorts data descending when sort is clicked second time', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enableSorting: true,
        }),
      )

      act(() => {
        result.current.handleSort('name')
      })

      act(() => {
        result.current.handleSort('name')
      })

      expect(result.current.sortColumn).toBe('name')
      expect(result.current.sortDirection).toBe('desc')
      expect(result.current.displayedData[0].name).toBe('Eve')
      expect(result.current.displayedData[4].name).toBe('Alice')
    })

    it('clears sort when sort is clicked third time', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enableSorting: true,
        }),
      )

      act(() => {
        result.current.handleSort('name')
      })

      act(() => {
        result.current.handleSort('name')
      })

      act(() => {
        result.current.handleSort('name')
      })

      expect(result.current.sortColumn).toBe(null)
      expect(result.current.sortDirection).toBe('asc')
    })

    it('sorts numbers correctly', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enableSorting: true,
        }),
      )

      act(() => {
        result.current.handleSort('age')
      })

      expect(result.current.displayedData[0].age).toBe(25)
      expect(result.current.displayedData[4].age).toBe(35)
    })

    it('uses default sort column', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enableSorting: true,
          defaultSortColumn: 'age',
          defaultSortDirection: 'desc',
        }),
      )

      expect(result.current.sortColumn).toBe('age')
      expect(result.current.sortDirection).toBe('desc')
      expect(result.current.displayedData[0].age).toBe(35)
    })
  })

  describe('search', () => {
    it('filters data based on search query', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enableSearch: true,
          searchFields: ['name', 'status'],
        }),
      )

      act(() => {
        result.current.setSearchQuery('active')
      })

      // Should find 3 agents with 'active' status
      expect(result.current.filteredRows).toBeGreaterThanOrEqual(3)
      expect(result.current.displayedData.length).toBeGreaterThanOrEqual(3)
    })

    it('is case insensitive', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enableSearch: true,
          searchFields: ['name'],
        }),
      )

      act(() => {
        result.current.setSearchQuery('ALICE')
      })

      expect(result.current.filteredRows).toBe(1)
      expect(result.current.displayedData[0].name).toBe('Alice')
    })

    it('searches across multiple fields', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enableSearch: true,
          searchFields: ['name', 'status'],
        }),
      )

      act(() => {
        result.current.setSearchQuery('Bob')
      })

      expect(result.current.filteredRows).toBe(1)
    })

    it('resets page to 0 when search changes', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enableSearch: true,
          enablePagination: true,
          defaultPageSize: 2,
        }),
      )

      act(() => {
        result.current.setPage(2)
      })

      expect(result.current.page).toBe(2)

      act(() => {
        result.current.setSearchQuery('Alice')
      })

      expect(result.current.page).toBe(0)
    })
  })

  describe('pagination', () => {
    it('paginates data correctly', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enablePagination: true,
          defaultPageSize: 2,
        }),
      )

      expect(result.current.displayedData).toHaveLength(2)
      expect(result.current.totalPages).toBe(3)
    })

    it('changes page size', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enablePagination: true,
          defaultPageSize: 2,
        }),
      )

      act(() => {
        result.current.setPageSize(3)
      })

      expect(result.current.displayedData).toHaveLength(3)
      expect(result.current.totalPages).toBe(2)
    })

    it('resets to page 0 when page size changes', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enablePagination: true,
          defaultPageSize: 2,
        }),
      )

      act(() => {
        result.current.setPage(1)
      })

      expect(result.current.page).toBe(1)

      act(() => {
        result.current.setPageSize(5)
      })

      expect(result.current.page).toBe(0)
    })

    it('navigates between pages', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enablePagination: true,
          defaultPageSize: 2,
        }),
      )

      expect(result.current.displayedData[0].name).toBe('Alice')

      act(() => {
        result.current.setPage(1)
      })

      expect(result.current.displayedData[0].name).toBe('Charlie')
    })
  })

  describe('selection', () => {
    it('toggles row selection', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
        }),
      )

      act(() => {
        result.current.toggleRowSelection('1')
      })

      expect(result.current.selectedRowKeys.has('1')).toBe(true)

      act(() => {
        result.current.toggleRowSelection('1')
      })

      expect(result.current.selectedRowKeys.has('1')).toBe(false)
    })

    it('selects all rows', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
        }),
      )

      const allKeys = mockData.map((row) => row.id)

      act(() => {
        result.current.toggleAllSelection(allKeys)
      })

      expect(result.current.selectedRowKeys.size).toBe(5)
    })

    it('deselects all rows when all are selected', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
        }),
      )

      const allKeys = mockData.map((row) => row.id)

      act(() => {
        result.current.toggleAllSelection(allKeys)
      })

      act(() => {
        result.current.toggleAllSelection(allKeys)
      })

      expect(result.current.selectedRowKeys.size).toBe(0)
    })

    it('clears selection', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
        }),
      )

      act(() => {
        result.current.toggleRowSelection('1')
        result.current.toggleRowSelection('2')
      })

      expect(result.current.selectedRowKeys.size).toBe(2)

      act(() => {
        result.current.clearSelection()
      })

      expect(result.current.selectedRowKeys.size).toBe(0)
    })
  })

  describe('combined features', () => {
    it('sorts and filters data', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enableSorting: true,
          enableSearch: true,
          searchFields: ['status'],
        }),
      )

      act(() => {
        result.current.setSearchQuery('active')
      })

      act(() => {
        result.current.handleSort('name')
      })

      // Should have filtered to 'active' status agents
      expect(result.current.filteredRows).toBeGreaterThanOrEqual(3)
      expect(result.current.displayedData.length).toBeGreaterThanOrEqual(3)
    })

    it('sorts, filters, and paginates data', () => {
      const {result} = renderHook(() =>
        useTableState({
          data: mockData,
          columns: mockColumns,
          enableSorting: true,
          enableSearch: true,
          enablePagination: true,
          defaultPageSize: 2,
          searchFields: ['status'],
        }),
      )

      act(() => {
        result.current.setSearchQuery('active')
      })

      act(() => {
        result.current.handleSort('age')
      })

      // Should have filtered rows
      expect(result.current.filteredRows).toBeGreaterThanOrEqual(3)
      // Should be paginated to 2 rows per page
      expect(result.current.displayedData.length).toBeLessThanOrEqual(2)
    })
  })
})
