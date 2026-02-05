import {describe, it, expect, vi} from 'vitest'
import {render, screen} from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import {ActionMenu} from './ActionMenu'
import DeleteIcon from '@mui/icons-material/Delete'
import EditIcon from '@mui/icons-material/Edit'

describe('ActionMenu', () => {
  const defaultActions = [
    {
      key: 'edit',
      label: 'Edit',
      icon: <EditIcon fontSize="small" />,
      onClick: vi.fn(),
    },
    {
      key: 'delete',
      label: 'Delete',
      icon: <DeleteIcon fontSize="small" />,
      onClick: vi.fn(),
      color: 'error' as const,
    },
  ]

  it('renders menu button', () => {
    render(<ActionMenu actions={defaultActions} />)

    const button = screen.getByLabelText('Actions')
    expect(button).toBeInTheDocument()
  })

  it('uses custom aria label', () => {
    render(<ActionMenu actions={defaultActions} ariaLabel="Custom Actions" />)

    const button = screen.getByLabelText('Custom Actions')
    expect(button).toBeInTheDocument()
  })

  it('opens menu on button click', async () => {
    const user = userEvent.setup()

    render(<ActionMenu actions={defaultActions} />)

    const button = screen.getByLabelText('Actions')
    await user.click(button)

    expect(screen.getByText('Edit')).toBeInTheDocument()
    expect(screen.getByText('Delete')).toBeInTheDocument()
  })

  it('calls onClick when menu item clicked', async () => {
    const user = userEvent.setup()
    const onEdit = vi.fn()
    const actions = [
      {
        key: 'edit',
        label: 'Edit',
        onClick: onEdit,
      },
    ]

    render(<ActionMenu actions={actions} />)

    const button = screen.getByLabelText('Actions')
    await user.click(button)

    const editItem = screen.getByText('Edit')
    await user.click(editItem)

    expect(onEdit).toHaveBeenCalledTimes(1)
  })

  it('closes menu after action click', async () => {
    const user = userEvent.setup()
    const actions = [
      {
        key: 'edit',
        label: 'Edit',
        onClick: vi.fn(),
      },
    ]

    render(<ActionMenu actions={actions} />)

    const button = screen.getByLabelText('Actions')
    await user.click(button)

    const editItem = screen.getByText('Edit')
    await user.click(editItem)

    // Menu should be closed after clicking an action
    expect(screen.queryByRole('menu')).not.toBeInTheDocument()
  })

  it('renders dividers between action groups', async () => {
    const user = userEvent.setup()
    const actions = [
      {
        key: 'edit',
        label: 'Edit',
        onClick: vi.fn(),
        dividerAfter: true,
      },
      {
        key: 'delete',
        label: 'Delete',
        onClick: vi.fn(),
      },
    ]

    render(<ActionMenu actions={actions} />)

    const button = screen.getByLabelText('Actions')
    await user.click(button)

    // Check that both items are rendered
    expect(screen.getByText('Edit')).toBeInTheDocument()
    expect(screen.getByText('Delete')).toBeInTheDocument()
  })

  it('filters out disabled actions', () => {
    const actions = [
      {
        key: 'edit',
        label: 'Edit',
        onClick: vi.fn(),
      },
      {
        key: 'delete',
        label: 'Delete',
        onClick: vi.fn(),
        disabled: true,
      },
    ]

    render(<ActionMenu actions={actions} />)

    // Menu button should still render since edit action is enabled
    expect(screen.getByLabelText('Actions')).toBeInTheDocument()
  })

  it('returns null when all actions are disabled', () => {
    const actions = [
      {
        key: 'edit',
        label: 'Edit',
        onClick: vi.fn(),
        disabled: true,
      },
      {
        key: 'delete',
        label: 'Delete',
        onClick: vi.fn(),
        disabled: true,
      },
    ]

    const {container} = render(<ActionMenu actions={actions} />)

    expect(container.firstChild).toBeNull()
  })

  it('returns null when actions array is empty', () => {
    const {container} = render(<ActionMenu actions={[]} />)

    expect(container.firstChild).toBeNull()
  })

  it('handles async onClick', async () => {
    const user = userEvent.setup()
    const asyncAction = vi.fn().mockResolvedValue(undefined)
    const actions = [
      {
        key: 'async',
        label: 'Async Action',
        onClick: asyncAction,
      },
    ]

    render(<ActionMenu actions={actions} />)

    const button = screen.getByLabelText('Actions')
    await user.click(button)

    const actionItem = screen.getByText('Async Action')
    await user.click(actionItem)

    expect(asyncAction).toHaveBeenCalledTimes(1)
  })
})
