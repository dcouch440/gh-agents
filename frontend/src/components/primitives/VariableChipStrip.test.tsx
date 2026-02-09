import {describe, it, expect, vi} from 'vitest'
import {render, screen} from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import {VariableChipStrip} from './VariableChipStrip'
import type {VariableCompletion} from '@/utils/variableContext'

const makeCompletion = (
  varName: string,
  section: string,
): VariableCompletion => ({
  label: `{${varName}}`,
  displayLabel: varName,
  detail: `string — from ${section}`,
  section,
})

describe('VariableChipStrip', () => {
  it('renders nothing when completions array is empty', () => {
    const {container} = render(
      <VariableChipStrip completions={[]} onCopy={null} />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('renders a chip for each completion', () => {
    const completions = [
      makeCompletion('result.summary', 'Parse'),
      makeCompletion('result.title', 'Parse'),
    ]
    render(<VariableChipStrip completions={completions} onCopy={null} />)

    expect(screen.getByText('{result.summary}')).toBeInTheDocument()
    expect(screen.getByText('{result.title}')).toBeInTheDocument()
  })

  it('groups chips by section', () => {
    const completions = [
      makeCompletion('a.field', 'StepA'),
      makeCompletion('b.field', 'StepB'),
    ]
    render(<VariableChipStrip completions={completions} onCopy={null} />)

    expect(screen.getByText('StepA')).toBeInTheDocument()
    expect(screen.getByText('StepB')).toBeInTheDocument()
  })

  it('calls onCopy with the label when a chip is clicked', async () => {
    const user = userEvent.setup()
    const onCopy = vi.fn()
    const completions = [makeCompletion('output.name', 'Extract')]

    render(<VariableChipStrip completions={completions} onCopy={onCopy} />)

    await user.click(screen.getByText('{output.name}'))
    expect(onCopy).toHaveBeenCalledOnce()
    expect(onCopy).toHaveBeenCalledWith('{output.name}')
  })

  it('does not crash when onCopy is null and chip is clicked', async () => {
    const user = userEvent.setup()
    const completions = [makeCompletion('data.value', 'Source')]

    render(<VariableChipStrip completions={completions} onCopy={null} />)

    await user.click(screen.getByText('{data.value}'))
  })
})
