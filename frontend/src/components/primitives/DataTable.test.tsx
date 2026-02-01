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

  it('applies sortable class when column is sortable and onSort provided', () => {
    const { container } = render(
      <DataTable columns={columns} rows={rows} rowKey={(r) => r.id} onSort={() => undefined} />,
    )
    const ths = container.querySelectorAll('th')
    expect(ths[0]).not.toHaveClass('th--sortable')
    expect(ths[1]).toHaveClass('th--sortable')
  })

  it('does not apply sortable class when onSort is not provided', () => {
    const { container } = render(
      <DataTable columns={columns} rows={rows} rowKey={(r) => r.id} />,
    )
    const ths = container.querySelectorAll('th')
    expect(ths[1]).not.toHaveClass('th--sortable')
  })

  it('calls onSort with column key when sortable header clicked', () => {
    const onSort = vi.fn()
    render(
      <DataTable columns={columns} rows={rows} rowKey={(r) => r.id} onSort={onSort} />,
    )
    fireEvent.click(screen.getByText('Score'))
    expect(onSort).toHaveBeenCalledWith('score')
  })

  it('shows sort direction indicator on active column', () => {
    render(
      <DataTable
        columns={columns}
        rows={rows}
        rowKey={(r) => r.id}
        onSort={() => undefined}
        sortColumn="score"
        sortDirection="asc"
      />,
    )
    expect(screen.getByText('Score ↑')).toBeInTheDocument()
  })

  it('renders empty tbody when no rows', () => {
    const { container } = render(
      <DataTable columns={columns} rows={[]} rowKey={(r) => r.id} />,
    )
    expect(container.querySelectorAll('tbody tr')).toHaveLength(0)
  })
})
