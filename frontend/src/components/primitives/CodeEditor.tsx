import { useEffect, useRef, useCallback } from 'react'
import { Box } from '@mui/material'
import { EditorView, placeholder as cmPlaceholder, keymap, lineNumbers, tooltips } from '@codemirror/view'
import { EditorState, type Extension } from '@codemirror/state'
import { markdown } from '@codemirror/lang-markdown'
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands'
import { completionKeymap } from '@codemirror/autocomplete'
import { syntaxHighlighting } from '@codemirror/language'
import { oneDarkTheme, oneDarkHighlightStyle } from '@codemirror/theme-one-dark'

type CodeEditorProps = {
  value: string
  onChange: (value: string) => void
  language?: 'markdown'
  placeholder?: string
  readOnly?: boolean
  showLineNumbers?: boolean
  height?: string
  className?: string
  extensions?: Extension[]
  editorViewRef?: (view: EditorView | null) => void
}

function CodeEditor({
  value,
  onChange,
  language = 'markdown',
  placeholder,
  readOnly = false,
  showLineNumbers = false,
  height = '300px',
  className,
  extensions: extraExtensions = [],
  editorViewRef,
}: CodeEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const viewRef = useRef<EditorView | null>(null)
  const isUpdatingRef = useRef(false)
  const onChangeRef = useRef(onChange)
  onChangeRef.current = onChange

  const getLanguageExtension = useCallback((): Extension[] => {
    const lang: string = language
    if (lang === 'markdown') return [markdown()]
    return []
  }, [language])

  useEffect(() => {
    if (!containerRef.current) return

    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged && !isUpdatingRef.current) {
        onChangeRef.current(update.state.doc.toString())
      }
    })

    const baseExtensions: Extension[] = [
      keymap.of([...completionKeymap, ...defaultKeymap, ...historyKeymap]),
      history(),
      oneDarkTheme,
      syntaxHighlighting(oneDarkHighlightStyle),
      EditorView.lineWrapping,
      tooltips({ parent: document.body }),
      updateListener,
      ...getLanguageExtension(),
      ...extraExtensions,
    ]

    if (placeholder) {
      baseExtensions.push(cmPlaceholder(placeholder))
    }

    if (showLineNumbers) {
      baseExtensions.push(lineNumbers())
    }

    if (readOnly) {
      baseExtensions.push(EditorState.readOnly.of(true))
    }

    const state = EditorState.create({
      doc: value,
      extensions: baseExtensions,
    })

    const view = new EditorView({
      state,
      parent: containerRef.current,
    })

    viewRef.current = view
    editorViewRef?.(view)

    return () => {
      view.destroy()
      viewRef.current = null
      editorViewRef?.(null)
    }
    // Only run on mount/unmount
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // Sync external value changes
  useEffect(() => {
    const view = viewRef.current
    if (!view) return

    const currentDoc = view.state.doc.toString()
    if (currentDoc === value) return

    isUpdatingRef.current = true
    view.dispatch({
      changes: { from: 0, to: currentDoc.length, insert: value },
    })
    isUpdatingRef.current = false
  }, [value])

  return (
    <Box
      ref={containerRef}
      className={className}
      sx={{
        height,
        border: 1,
        borderColor: 'divider',
        borderRadius: 1,
        overflow: 'hidden',
        fontFamily: 'monospace',
        '&:focus-within': {
          borderColor: 'primary.main',
        },
        '& .cm-editor': {
          height: '100%',
        },
        '& .cm-scroller': {
          overflow: 'auto',
        },
      }}
    />
  )
}

export { CodeEditor }
export type { CodeEditorProps }
