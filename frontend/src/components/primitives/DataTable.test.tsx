import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { DataTable } from './DataTable'
import type { Column } from './DataTable'

type TestRow = { id: string; name: string; score: number }

const rows: TestRow[] = [
  { id: '1', name: 'Alice', score: 90 },
  { id: '2', name: 'Bob', score: 85 },
]

const columns: Column<TestRow>[] = [
  { key: 'name', header: 'Name', render: (r) => r.name },
  { key: 'score', header: 'Score', sortable: true, render: (r) => r.score },
]

describe('DataTable', () => {
  it('renders column headers', () => {
    render(<DataTable columns={columns} rows={rows} rowKey={(r) => r.id} />)
    expect(screen.getByText('Name')).toBeInTheDocument()
    expect(screen.getByText('Score')).toBeInTheDocument()
  })

  it('renders row data', () => {
    render(<DataTable columns={columns} rows={rows} rowKey={(r) => r.id} />)
    expect(screen.getByText('Alice')).toBeInTheDocument()
    expect(screen.getByText('85')).toBeInTheDocument()
  })

  it('renders sortable column with TableSortLabel when onSort provided', () => {
    const { container } = render(<DataTable columns={columns} rows={rows} rowKey={(r) => r.id} onSort={() => undefined} />)
    const sortLabels = container.querySelectorAll('.MuiTableSortLabel-root')
    expect(sortLabels).toHaveLength(1)
  })

  it('does not render TableSortLabel when onSort is not provided', () => {
    const { container } = render(<DataTable columns={columns} rows={rows} rowKey={(r) => r.id} />)
    const sortLabels = container.querySelectorAll('.MuiTableSortLabel-root')
    expect(sortLabels).toHaveLength(0)
  })

  it('calls onSort with column key when sortable header clicked', () => {
    const onSort = vi.fn()
    render(<DataTable columns={columns} rows={rows} rowKey={(r) => r.id} onSort={onSort} />)
    fireEvent.click(screen.getByText('Score'))
    expect(onSort).toHaveBeenCalledWith('score')
  })

  it('shows active sort label on sorted column', () => {
    const { container } = render(
      <DataTable columns={columns} rows={rows} rowKey={(r) => r.id} onSort={() => undefined} sortColumn="score" sortDirection="asc" />,
    )
    const activeLabel = container.querySelector('.MuiTableSortLabel-root.Mui-active')
    expect(activeLabel).toBeInTheDocument()
  })

  it('renders empty tbody when no rows', () => {
    const { container } = render(<DataTable columns={columns} rows={[]} rowKey={(r) => r.id} />)
    expect(container.querySelectorAll('tbody tr')).toHaveLength(0)
  })
})
