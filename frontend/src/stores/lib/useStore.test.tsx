import { render, screen, act } from '@testing-library/react'
import { renderHook } from '@testing-library/react'
import { useStore } from './useStore'
import { createStore } from './createStore'
import { shallow } from './shallow'

type TestState = {
  a: number
  b: number
  nested: { x: number }
}

const makeStore = () => createStore<TestState>(() => ({ a: 1, b: 2, nested: { x: 10 } }))

describe('useStore', () => {
  it('returns selected slice of state', () => {
    const store = makeStore()
    const { result } = renderHook(() => useStore(store, (s) => s.a))
    expect(result.current).toBe(1)
  })

  it('re-renders when selected slice changes', () => {
    const store = makeStore()
    const { result } = renderHook(() => useStore(store, (s) => s.a))

    act(() => {
      store.setState({ a: 42 })
    })

    expect(result.current).toBe(42)
  })

  it('does not re-render when unrelated state changes', () => {
    const store = makeStore()
    let renderCount = 0

    function TestComponent() {
      const a = useStore(store, (s) => s.a)
      renderCount++
      return <div data-testid="a">{a}</div>
    }

    render(<TestComponent />)
    expect(screen.getByTestId('a')).toHaveTextContent('1')
    expect(renderCount).toBe(1)

    // Update b — should NOT cause re-render since we only select a
    act(() => {
      store.setState({ b: 999 })
    })

    expect(renderCount).toBe(1)
    expect(screen.getByTestId('a')).toHaveTextContent('1')
  })

  it('supports custom equality function', () => {
    const store = makeStore()
    let renderCount = 0

    function TestComponent() {
      const slice = useStore(store, (s) => ({ a: s.a, b: s.b }), shallow)
      renderCount++
      return <div data-testid="sum">{slice.a + slice.b}</div>
    }

    render(<TestComponent />)
    expect(renderCount).toBe(1)

    // setState creates new object but shallow-equal — should NOT re-render
    act(() => {
      store.setState({ a: 1, b: 2 }) // Same values
    })

    expect(renderCount).toBe(1)

    // Now actually change a value
    act(() => {
      store.setState({ a: 10 })
    })

    expect(renderCount).toBe(2)
    expect(screen.getByTestId('sum')).toHaveTextContent('12')
  })

  it('returns same reference when equality check passes', () => {
    const store = makeStore()
    const refs: number[] = []

    function TestComponent() {
      const a = useStore(store, (s) => s.a)
      refs.push(a)
      return <div>{a}</div>
    }

    render(<TestComponent />)

    // Update unrelated field
    act(() => {
      store.setState({ b: 100 })
    })

    // a selector returns 1 both times — same primitive
    expect(refs.every((r) => r === 1)).toBe(true)
  })

  it('works with multiple components subscribing to same store', () => {
    const store = makeStore()

    function ComponentA() {
      const a = useStore(store, (s) => s.a)
      return <div data-testid="comp-a">{a}</div>
    }

    function ComponentB() {
      const b = useStore(store, (s) => s.b)
      return <div data-testid="comp-b">{b}</div>
    }

    render(
      <>
        <ComponentA />
        <ComponentB />
      </>,
    )

    expect(screen.getByTestId('comp-a')).toHaveTextContent('1')
    expect(screen.getByTestId('comp-b')).toHaveTextContent('2')

    act(() => {
      store.setState({ a: 50 })
    })

    expect(screen.getByTestId('comp-a')).toHaveTextContent('50')
    expect(screen.getByTestId('comp-b')).toHaveTextContent('2')
  })
})
