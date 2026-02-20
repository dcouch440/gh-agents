import { EditorView } from '@codemirror/view'
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language'
import { tags } from '@lezer/highlight'
import { oneDarkTheme, oneDarkHighlightStyle } from '@codemirror/theme-one-dark'
import type { Extension } from '@codemirror/state'

/* ── Light earthy theme ─────────────────────────────────── */

const lightEditorTheme = EditorView.theme(
  {
    '&': {
      backgroundColor: '#F9F6F1',
      color: '#2D1B0E',
    },
    '.cm-content': {
      caretColor: '#5a8a6e',
    },
    '.cm-cursor, .cm-dropCursor': {
      borderLeftColor: '#5a8a6e',
    },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
      backgroundColor: 'rgba(90, 138, 110, 0.20)',
    },
    '.cm-panels': {
      backgroundColor: '#F4F0EA',
      color: '#2D1B0E',
    },
    '.cm-panels.cm-panels-top': {
      borderBottom: '1px solid rgba(45, 27, 14, 0.10)',
    },
    '.cm-panels.cm-panels-bottom': {
      borderTop: '1px solid rgba(45, 27, 14, 0.10)',
    },
    '.cm-searchMatch': {
      backgroundColor: 'rgba(90, 138, 110, 0.18)',
      outline: '1px solid rgba(90, 138, 110, 0.35)',
    },
    '.cm-searchMatch.cm-searchMatch-selected': {
      backgroundColor: 'rgba(78, 138, 90, 0.22)',
    },
    '.cm-activeLine': {
      backgroundColor: 'rgba(45, 27, 14, 0.03)',
    },
    '.cm-selectionMatch': {
      backgroundColor: 'rgba(90, 138, 110, 0.12)',
    },
    '&.cm-focused .cm-matchingBracket, &.cm-focused .cm-nonmatchingBracket': {
      backgroundColor: 'rgba(78, 138, 90, 0.28)',
    },
    '.cm-gutters': {
      backgroundColor: '#F4F0EA',
      color: '#A39283',
      borderRight: '1px solid rgba(45, 27, 14, 0.08)',
    },
    '.cm-activeLineGutter': {
      backgroundColor: 'rgba(45, 27, 14, 0.04)',
    },
    '.cm-foldPlaceholder': {
      backgroundColor: 'rgba(45, 27, 14, 0.06)',
      border: 'none',
      color: '#6B5742',
    },
    '.cm-tooltip': {
      backgroundColor: '#FEFCFA',
      border: '1px solid rgba(45, 27, 14, 0.12)',
    },
    '.cm-tooltip .cm-tooltip-arrow:before': {
      borderTopColor: 'rgba(45, 27, 14, 0.12)',
      borderBottomColor: 'rgba(45, 27, 14, 0.12)',
    },
    '.cm-tooltip .cm-tooltip-arrow:after': {
      borderTopColor: '#FEFCFA',
      borderBottomColor: '#FEFCFA',
    },
    '.cm-tooltip-autocomplete': {
      '& > ul > li[aria-selected]': {
        backgroundColor: 'rgba(90, 138, 110, 0.15)',
        color: '#2D1B0E',
      },
    },
    '.cm-placeholder': {
      color: '#A39283',
    },
  },
  { dark: false },
)

const lightHighlightStyle = HighlightStyle.define([
  { tag: tags.keyword, color: '#D47830' },
  { tag: [tags.name, tags.deleted, tags.character, tags.propertyName, tags.macroName], color: '#FF964F' },
  { tag: [tags.function(tags.variableName), tags.labelName], color: '#D47830' },
  { tag: [tags.color, tags.constant(tags.name), tags.standard(tags.name)], color: '#725438' },
  { tag: [tags.definition(tags.name), tags.separator], color: '#2D1B0E' },
  {
    tag: [tags.typeName, tags.className, tags.number, tags.changed, tags.annotation, tags.modifier, tags.self, tags.namespace],
    color: '#955C0A',
  },
  {
    tag: [tags.operator, tags.operatorKeyword, tags.url, tags.escape, tags.regexp, tags.link, tags.special(tags.string)],
    color: '#725438',
  },
  { tag: [tags.meta, tags.comment], color: '#A39283' },
  { tag: tags.strong, fontWeight: 'bold' },
  { tag: tags.emphasis, fontStyle: 'italic' },
  { tag: tags.strikethrough, textDecoration: 'line-through' },
  { tag: tags.link, color: '#3B7046', textDecoration: 'underline' },
  { tag: tags.heading, fontWeight: 'bold', color: '#D47830' },
  { tag: [tags.atom, tags.bool, tags.special(tags.variableName)], color: '#955C0A' },
  { tag: [tags.processingInstruction, tags.string, tags.inserted], color: '#3B7046' },
  { tag: tags.invalid, color: '#BF3326' },
])

/* ── Public API ──────────────────────────────────────────── */

const getEditorThemeExtensions = (mode: 'light' | 'dark'): Extension[] => {
  if (mode === 'light') {
    return [lightEditorTheme, syntaxHighlighting(lightHighlightStyle)]
  }
  return [oneDarkTheme, syntaxHighlighting(oneDarkHighlightStyle)]
}

export { getEditorThemeExtensions }
