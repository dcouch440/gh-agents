import { EditorView } from '@codemirror/view'
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language'
import { tags } from '@lezer/highlight'
import { oneDarkHighlightStyle } from '@codemirror/theme-one-dark'
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

/* ── Dark theme (parameterised background) ────────────── */

const createDarkEditorTheme = (bg: string, textColor: string, accentColor: string) =>
  EditorView.theme(
    {
      '&': {
        backgroundColor: bg,
        color: textColor,
      },
      '.cm-content': {
        caretColor: accentColor,
      },
      '.cm-cursor, .cm-dropCursor': {
        borderLeftColor: accentColor,
      },
      '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
        backgroundColor: `${accentColor}30`,
      },
      '.cm-panels': {
        backgroundColor: bg,
        color: textColor,
      },
      '.cm-panels.cm-panels-top': {
        borderBottom: '1px solid rgba(255, 255, 255, 0.08)',
      },
      '.cm-panels.cm-panels-bottom': {
        borderTop: '1px solid rgba(255, 255, 255, 0.08)',
      },
      '.cm-searchMatch': {
        backgroundColor: `${accentColor}25`,
        outline: `1px solid ${accentColor}50`,
      },
      '.cm-searchMatch.cm-searchMatch-selected': {
        backgroundColor: `${accentColor}35`,
      },
      '.cm-activeLine': {
        backgroundColor: 'rgba(255, 255, 255, 0.03)',
      },
      '.cm-selectionMatch': {
        backgroundColor: `${accentColor}18`,
      },
      '&.cm-focused .cm-matchingBracket, &.cm-focused .cm-nonmatchingBracket': {
        backgroundColor: `${accentColor}35`,
      },
      '.cm-gutters': {
        backgroundColor: bg,
        color: 'rgba(255, 255, 255, 0.3)',
        borderRight: '1px solid rgba(255, 255, 255, 0.06)',
      },
      '.cm-activeLineGutter': {
        backgroundColor: 'rgba(255, 255, 255, 0.04)',
      },
      '.cm-foldPlaceholder': {
        backgroundColor: 'rgba(255, 255, 255, 0.06)',
        border: 'none',
        color: 'rgba(255, 255, 255, 0.5)',
      },
      '.cm-tooltip': {
        backgroundColor: bg,
        border: '1px solid rgba(255, 255, 255, 0.1)',
      },
      '.cm-tooltip .cm-tooltip-arrow:before': {
        borderTopColor: 'rgba(255, 255, 255, 0.1)',
        borderBottomColor: 'rgba(255, 255, 255, 0.1)',
      },
      '.cm-tooltip .cm-tooltip-arrow:after': {
        borderTopColor: bg,
        borderBottomColor: bg,
      },
      '.cm-tooltip-autocomplete': {
        '& > ul > li[aria-selected]': {
          backgroundColor: `${accentColor}20`,
          color: textColor,
        },
      },
      '.cm-placeholder': {
        color: 'rgba(255, 255, 255, 0.35)',
      },
    },
    { dark: true },
  )

/* ── Public API ──────────────────────────────────────────── */

type EditorThemeTokens = {
  bgEditor: string
  textPrimary: string
  accent: string
}

const getEditorThemeExtensions = (mode: 'light' | 'dark', tokens?: EditorThemeTokens): Extension[] => {
  if (mode === 'light') {
    return [lightEditorTheme, syntaxHighlighting(lightHighlightStyle)]
  }
  const bg = tokens?.bgEditor ?? '#161b22'
  const text = tokens?.textPrimary ?? '#f0f6fc'
  const accent = tokens?.accent ?? '#3b82f6'
  return [createDarkEditorTheme(bg, text, accent), syntaxHighlighting(oneDarkHighlightStyle)]
}

export { getEditorThemeExtensions }
export type { EditorThemeTokens }
