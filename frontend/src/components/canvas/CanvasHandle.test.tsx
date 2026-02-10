import { describe, it, expect, vi } from 'vitest'
import { render } from '@/test/render'
import { CanvasHandle } from './CanvasHandle'

vi.mock('@xyflow/react', () => ({
  Handle: (props: Record<string, unknown>) => {
    const style = props.style as Record<string, unknown> | undefined
    return (
      <div
        data-testid="handle"
        data-type={props.type}
        data-position={props.position}
        data-id={props.id ?? ''}
        style={style}
      />
    )
  },
  Position: { Left: 'left', Right: 'right', Top: 'top', Bottom: 'bottom' },
}))

describe('CanvasHandle', () => {
  it('renders standard variant with correct size', () => {
    const { getByTestId } = render(
      <CanvasHandle type="source" position={'right' as never} color="#ff0000" />,
    )
    const el = getByTestId('handle')
    expect(el.style.width).toBe('12px')
    expect(el.style.height).toBe('12px')
  })

  it('renders small variant with correct size', () => {
    const { getByTestId } = render(
      <CanvasHandle type="source" position={'right' as never} color="#ff0000" variant="small" />,
    )
    const el = getByTestId('handle')
    expect(el.style.width).toBe('8px')
    expect(el.style.height).toBe('8px')
  })

  it('renders passive variant with pointerEvents none', () => {
    const { getByTestId } = render(
      <CanvasHandle type="target" position={'bottom' as never} color="#ff0000" variant="passive" />,
    )
    const el = getByTestId('handle')
    expect(el.style.width).toBe('8px')
    expect(el.style.height).toBe('8px')
    expect(el.style.pointerEvents).toBe('none')
  })

  it('does not set pointerEvents for standard variant', () => {
    const { getByTestId } = render(
      <CanvasHandle type="source" position={'right' as never} color="#ff0000" />,
    )
    const el = getByTestId('handle')
    expect(el.style.pointerEvents).toBe('')
  })

  it('passes id prop through to Handle', () => {
    const { getByTestId } = render(
      <CanvasHandle type="source" position={'top' as never} color="#ff0000" id="documents" />,
    )
    const el = getByTestId('handle')
    expect(el.getAttribute('data-id')).toBe('documents')
  })

  it('passes type and position through to Handle', () => {
    const { getByTestId } = render(
      <CanvasHandle type="target" position={'left' as never} color="#ff0000" />,
    )
    const el = getByTestId('handle')
    expect(el.getAttribute('data-type')).toBe('target')
    expect(el.getAttribute('data-position')).toBe('left')
  })

  it('uses color prop for background', () => {
    const { getByTestId } = render(
      <CanvasHandle type="source" position={'right' as never} color="#D4793E" />,
    )
    const el = getByTestId('handle')
    expect(el.style.background).toBe('rgb(212, 121, 62)')
  })
})
