import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { CodeEditor } from './CodeEditor'

describe('CodeEditor', () => {
  it('renders a container element', () => {
    const { container } = render(
      <CodeEditor value="" onChange={() => {}} />
    )
    // The outer Box renders as a div; it is the first child
    const editor = container.firstChild as HTMLElement
    expect(editor).toBeInTheDocument()
  })

  it('applies custom className', () => {
    const { container } = render(
      <CodeEditor value="" onChange={() => {}} className="custom" />
    )
    const editor = container.firstChild as HTMLElement
    expect(editor).toHaveClass('custom')
  })

  it('applies custom height', () => {
    const { container, rerender } = render(
      <CodeEditor value="" onChange={() => {}} height="500px" />
    )
    const editor = container.firstChild as HTMLElement
    const classA = editor.className
    // Re-render with a different height to verify the sx prop updates
    rerender(<CodeEditor value="" onChange={() => {}} height="800px" />)
    const classB = editor.className
    expect(classA).not.toBe(classB)
  })

  it('initializes CodeMirror editor inside container', () => {
    const { container } = render(
      <CodeEditor value="hello world" onChange={() => {}} />
    )
    const cmEditor = container.querySelector('.cm-editor')
    expect(cmEditor).toBeInTheDocument()
  })

  it('exposes EditorView via editorViewRef', () => {
    let capturedView: unknown = null
    render(
      <CodeEditor
        value="test"
        onChange={() => {}}
        editorViewRef={(v) => { capturedView = v }}
      />
    )
    expect(capturedView).not.toBeNull()
  })

  it('displays initial value in the editor', () => {
    const { container } = render(
      <CodeEditor value="# Hello" onChange={() => {}} />
    )
    expect(container.textContent).toContain('# Hello')
  })

  it('renders placeholder when provided and value is empty', () => {
    render(
      <CodeEditor value="" onChange={() => {}} placeholder="Type here..." />
    )
    expect(screen.getByText('Type here...')).toBeInTheDocument()
  })
})
