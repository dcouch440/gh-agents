import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@/test/render'
import { SharePickerPanel } from './SharePickerPanel'
import type { ShareableField } from '@/stores/shareStore'

const mocks = vi.hoisted(() => {
  const fields: ShareableField[] = [
    {
      key: 'name',
      label: 'Name',
      category: 'General',
      kind: 'shared-field',
      color: '#D4793E',
      chipKey: 'name',
      entity: { kind: 'shared-field', id: 's1::name', name: 'Node', summary: 'Node name', data: { fieldType: 'Node Name', value: 'Node' } },
    },
    {
      key: 'doc::d1',
      label: 'Design Spec',
      category: 'Documents',
      kind: 'document',
      color: '#D4793E',
      chipKey: 'doc',
      entity: { kind: 'document', id: 's1::doc::d1', name: 'Design Spec', summary: 'Document', data: { parentStepName: 'Node', description: '' } },
    },
  ]

  return {
    toggleField: vi.fn<(key: string) => void>(),
    fields,
  }
})

vi.mock('@/stores', () => {
  type StoreState = { availableFields: ShareableField[]; selectedKeys: Set<string> }

  const state: StoreState = {
    availableFields: mocks.fields,
    selectedKeys: new Set(['name']),
  }

  return {
    useStore: (_s: unknown, selector: (s: StoreState) => unknown) => selector(state),
    shareStore: {
      store: { getState: () => state, subscribe: () => () => {} },
      selectAvailableFields: (s: StoreState) => s.availableFields,
      selectSelectedKeys: (s: StoreState) => s.selectedKeys,
      toggleField: mocks.toggleField,
    },
  }
})

beforeEach(() => {
  vi.clearAllMocks()
})

describe('SharePickerPanel', () => {
  it('renders section header', () => {
    render(<SharePickerPanel stepId="s1" />)
    expect(screen.getByText('Share context')).toBeInTheDocument()
  })

  it('renders grouped field labels', () => {
    render(<SharePickerPanel stepId="s1" />)
    expect(screen.getByText('General')).toBeInTheDocument()
    expect(screen.getByText('Documents')).toBeInTheDocument()
    expect(screen.getByText('Name')).toBeInTheDocument()
    expect(screen.getByText('Design Spec')).toBeInTheDocument()
  })

  it('shows selected count', () => {
    render(<SharePickerPanel stepId="s1" />)
    expect(screen.getByText('1 of 2 selected')).toBeInTheDocument()
  })

  it('calls toggleField when a field row is clicked', () => {
    render(<SharePickerPanel stepId="s1" />)
    fireEvent.click(screen.getByText('Design Spec'))
    expect(mocks.toggleField).toHaveBeenCalledWith('doc::d1')
  })
})
