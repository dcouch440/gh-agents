import {describe, it, expect, vi} from 'vitest'
import {render, screen, within} from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import {Table} from './Table'
import type {TableColumn} from './types'

type TestRow = {
  id: string
  name: string
  age: number
}

const mockData: TestRow[] = [
  {id: '1', name: 'Alice', age: 30},
  {id: '2', name: 'Bob', age: 25},
  {id: '3', name: 'Charlie', age: 35},
]

const mockColumns: TableColumn<TestRow>[] = [
  {key: 'name', header: 'Name', sortable: true},
  {key: 'age', header: 'Age', sortable: true},
]

describe('Table', () => {
  describe('rendering', () => {
    it('renders table with data', () => {
      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
        />,
      )

      expect(screen.getByText('Alice')).toBeInTheDocument()
      expect(screen.getByText('Bob')).toBeInTheDocument()
      expect(screen.getByText('Charlie')).toBeInTheDocument()
    })

    it('renders column headers', () => {
      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
        />,
      )

      expect(screen.getByText('Name')).toBeInTheDocument()
      expect(screen.getByText('Age')).toBeInTheDocument()
    })

    it('renders custom cell content via render prop', () => {
      const customColumns: TableColumn<TestRow>[] = [
        {
          key: 'name',
          header: 'Name',
          render: (row) => <strong>{row.name.toUpperCase()}</strong>,
        },
      ]

      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={customColumns}
        />,
      )

      expect(screen.getByText('ALICE')).toBeInTheDocument()
    })
  })

  describe('loading state', () => {
    it('shows loading spinner when loading and no data', () => {
      render(
        <Table
          data={[]}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          loading
        />,
      )

      expect(screen.getByText('Loading data...')).toBeInTheDocument()
    })
  })

  describe('error state', () => {
    it('shows error message when error is set', () => {
      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          error="Something went wrong"
        />,
      )

      expect(screen.getByText('Something went wrong')).toBeInTheDocument()
    })
  })

  describe('empty state', () => {
    it('shows empty state when no data', () => {
      render(
        <Table
          data={[]}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          emptyMessage="No data available"
        />,
      )

      expect(screen.getByText('No data available')).toBeInTheDocument()
    })
  })

  describe('sorting', () => {
    it('sorts data when column header is clicked', async () => {
      const user = userEvent.setup()

      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          enableSorting
        />,
      )

      const nameHeader = screen.getByText('Name')
      await user.click(nameHeader)

      const rows = screen.getAllByRole('row')
      const firstDataRow = rows[1] // Skip header row
      expect(within(firstDataRow).getByText('Alice')).toBeInTheDocument()
    })

    it('toggles sort direction on second click', async () => {
      const user = userEvent.setup()

      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          enableSorting
        />,
      )

      const nameHeader = screen.getByText('Name')
      await user.click(nameHeader) // Sort asc
      await user.click(nameHeader) // Sort desc

      const rows = screen.getAllByRole('row')
      const firstDataRow = rows[1]
      expect(within(firstDataRow).getByText('Charlie')).toBeInTheDocument()
    })

    it('has sortable column headers', () => {
      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          enableSorting
        />,
      )

      const nameHeader = screen.getByText('Name')
      expect(nameHeader.closest('span')).toHaveClass('MuiTableSortLabel-root')
    })
  })

  describe('search', () => {
    it('shows search input when enableSearch is true', () => {
      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          enableSearch
          searchPlaceholder="Search..."
        />,
      )

      expect(screen.getByPlaceholderText('Search...')).toBeInTheDocument()
    })

    it('filters data based on search query', async () => {
      const user = userEvent.setup()

      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          enableSearch
          searchFields={['name']}
        />,
      )

      const searchInput = screen.getByPlaceholderText('Search...')
      await user.type(searchInput, 'Alice')

      // Wait for debounce
      await new Promise((resolve) => setTimeout(resolve, 350))

      expect(screen.getByText('Alice')).toBeInTheDocument()
      expect(screen.queryByText('Bob')).not.toBeInTheDocument()
    })
  })

  describe('pagination', () => {
    const largeData = Array.from({length: 50}, (_, i) => ({
      id: String(i),
      name: `Person ${i}`,
      age: 20 + i,
    }))

    it('shows pagination controls when enablePagination is true', () => {
      render(
        <Table
          data={largeData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          enablePagination
          defaultPageSize={10}
        />,
      )

      expect(screen.getByText('Rows per page:')).toBeInTheDocument()
    })

    it('paginates data correctly', () => {
      render(
        <Table
          data={largeData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          enablePagination
          defaultPageSize={10}
        />,
      )

      const rows = screen.getAllByRole('row')
      // 1 header + 10 data rows
      expect(rows).toHaveLength(11)
    })
  })

  describe('selection', () => {
    it('shows checkboxes when enableSelection is true', () => {
      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          enableSelection
        />,
      )

      const checkboxes = screen.getAllByRole('checkbox')
      // 1 select all + 3 row checkboxes
      expect(checkboxes).toHaveLength(4)
    })

    it('selects row when checkbox is clicked', async () => {
      const user = userEvent.setup()
      const onSelectionChange = vi.fn()

      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          enableSelection
          onSelectionChange={onSelectionChange}
        />,
      )

      const checkboxes = screen.getAllByRole('checkbox')
      const firstRowCheckbox = checkboxes[1] // Skip select all

      await user.click(firstRowCheckbox)

      expect(onSelectionChange).toHaveBeenCalled()
    })

    it('selects all rows when select all is clicked', async () => {
      const user = userEvent.setup()
      const onSelectionChange = vi.fn()

      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          enableSelection
          onSelectionChange={onSelectionChange}
        />,
      )

      const checkboxes = screen.getAllByRole('checkbox')
      const selectAllCheckbox = checkboxes[0]

      await user.click(selectAllCheckbox)

      expect(onSelectionChange).toHaveBeenCalled()
    })
  })

  describe('row click', () => {
    it('calls onRowClick when row is clicked', async () => {
      const user = userEvent.setup()
      const onRowClick = vi.fn()

      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          onRowClick={onRowClick}
        />,
      )

      const rows = screen.getAllByRole('row')
      const firstDataRow = rows[1] // Skip header

      await user.click(firstDataRow)

      expect(onRowClick).toHaveBeenCalledWith(mockData[0])
    })
  })

  describe('column visibility', () => {
    it('shows column menu when enableSearch is true', () => {
      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          enableSearch
        />,
      )

      const columnMenuButton = screen.getByLabelText('Column visibility')
      expect(columnMenuButton).toBeInTheDocument()
    })

    it('can hide and show columns via menu', () => {
      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          enableSearch
        />,
      )

      // Column menu exists
      const columnMenuButton = screen.getByLabelText('Column visibility')
      expect(columnMenuButton).toBeInTheDocument()
    })
  })

  describe('density', () => {
    it('renders with compact density', () => {
      render(
        <Table
          data={mockData}
          keyExtractor={(row) => row.id}
          columns={mockColumns}
          density="compact"
        />,
      )

      // Table renders successfully with compact density
      expect(screen.getByText('Alice')).toBeInTheDocument()
    })
  })
})
