import { Box } from '@mui/material'
import { useTerminalTheme } from '../hooks/useTerminalTheme'
import type { TableNode, InlineNode } from '../parser/types'

type TableRendererProps = {
  node: TableNode
}

const MAX_COL_WIDTH = 40

const getInlineTextLength = (nodes: InlineNode[]): number => {
  let len = 0
  for (const node of nodes) {
    switch (node.type) {
      case 'text':
        len += node.content.length
        break
      case 'inline_code':
        len += node.content.length
        break
      case 'strong':
      case 'emphasis':
      case 'strikethrough':
      case 'link':
        len += getInlineTextLength(node.children)
        break
      case 'image':
        len += (node.alt || 'image').length + 6 // "[img: ]"
        break
      case 'softbreak':
      case 'hardbreak':
        len += 1
        break
    }
  }
  return len
}

const getInlineText = (nodes: InlineNode[]): string => {
  let text = ''
  for (const node of nodes) {
    switch (node.type) {
      case 'text':
        text += node.content
        break
      case 'inline_code':
        text += node.content
        break
      case 'strong':
      case 'emphasis':
      case 'strikethrough':
      case 'link':
        text += getInlineText(node.children)
        break
      case 'image':
        text += `[img: ${node.alt || 'image'}]`
        break
      case 'softbreak':
      case 'hardbreak':
        text += ' '
        break
    }
  }
  return text
}

const padCell = (text: string, width: number, align: 'left' | 'center' | 'right' | null): string => {
  const truncated = text.length > width ? text.slice(0, width - 1) + '\u2026' : text
  const padding = width - truncated.length

  switch (align) {
    case 'right':
      return ' '.repeat(padding) + truncated
    case 'center': {
      const left = Math.floor(padding / 2)
      const right = padding - left
      return ' '.repeat(left) + truncated + ' '.repeat(right)
    }
    default:
      return truncated + ' '.repeat(padding)
  }
}

function TableRenderer({ node }: TableRendererProps) {
  const theme = useTerminalTheme()

  const colCount = node.header.cells.length
  const allRows = [node.header, ...node.body]

  // Compute column widths
  const colWidths: number[] = []
  for (let c = 0; c < colCount; c++) {
    let maxWidth = 0
    for (const row of allRows) {
      const cell = row.cells[c]
      if (cell) {
        const len = getInlineTextLength(cell.children)
        maxWidth = Math.max(maxWidth, len)
      }
    }
    colWidths.push(Math.min(Math.max(maxWidth, 3), MAX_COL_WIDTH))
  }

  // Build box-drawing lines
  const buildLine = (left: string, mid: string, right: string, fill: string): string => {
    const segments = colWidths.map((w) => fill.repeat(w + 2))
    return left + segments.join(mid) + right
  }

  const buildRow = (rowCells: { text: string; colIndex: number }[]): string => {
    const cells = colWidths.map((w, c) => {
      const cell = rowCells.find((rc) => rc.colIndex === c)
      const text = cell?.text ?? ''
      const align = node.alignments[c] ?? null
      return ' ' + padCell(text, w, align) + ' '
    })
    return '\u2502' + cells.join('\u2502') + '\u2502'
  }

  const topLine = buildLine('\u250c', '\u252c', '\u2510', '\u2500')
  const headerSep = buildLine('\u251c', '\u253c', '\u2524', '\u2500')
  const bottomLine = buildLine('\u2514', '\u2534', '\u2518', '\u2500')

  const headerRow = buildRow(
    node.header.cells.map((cell, c) => ({
      text: getInlineText(cell.children),
      colIndex: c,
    }))
  )

  const bodyRows = node.body.map((row) =>
    buildRow(
      row.cells.map((cell, c) => ({
        text: getInlineText(cell.children),
        colIndex: c,
      }))
    )
  )

  const lines = [topLine, headerRow, headerSep, ...bodyRows, bottomLine]

  return (
    <Box
      component="pre"
      role="table"
      aria-label="data table"
      sx={{
        m: 0,
        my: '0.4em',
        color: theme.tableBorder,
        fontFamily: 'inherit',
        fontSize: 'inherit',
        lineHeight: 1.4,
        overflowX: 'auto',
      }}
    >
      {lines.map((line, i) => {
        // Color header text differently
        if (i === 1) {
          return (
            <Box key={i} component="span" sx={{ color: theme.tableHeaderText }}>
              {line}{'\n'}
            </Box>
          )
        }
        return <span key={i}>{line}{'\n'}</span>
      })}
    </Box>
  )
}

export { TableRenderer }
export type { TableRendererProps }
