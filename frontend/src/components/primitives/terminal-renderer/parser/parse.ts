import MarkdownIt from 'markdown-it'
import type Token from 'markdown-it/lib/token.mjs'
import type {
  BlockNode,
  InlineNode,
  ListItemNode,
  TableRowNode,
  TableCellNode,
  HeadingNode,
} from './types'

// Module-level singleton — created once, shared across all calls
const md = new MarkdownIt({ breaks: true, linkify: true, typographer: false })

// Enable strikethrough (GFM ~~text~~)
md.enable('strikethrough')

let keyCounter = 0

const nextKey = (prefix: string): string => `${prefix}-${keyCounter++}`

const resetKeys = (): void => {
  keyCounter = 0
}

const parseInlineTokens = (tokens: Token[]): InlineNode[] => {
  const nodes: InlineNode[] = []
  const stack: InlineNode[][] = [nodes]

  for (const token of tokens) {
    const current = stack[stack.length - 1]!

    switch (token.type) {
      case 'text':
        if (token.content) {
          current.push({ type: 'text', key: nextKey('t'), content: token.content })
        }
        break

      case 'code_inline':
        current.push({ type: 'inline_code', key: nextKey('ic'), content: token.content })
        break

      case 'softbreak':
        current.push({ type: 'softbreak', key: nextKey('sb') })
        break

      case 'hardbreak':
        current.push({ type: 'hardbreak', key: nextKey('hb') })
        break

      case 'strong_open': {
        const children: InlineNode[] = []
        current.push({ type: 'strong', key: nextKey('b'), children })
        stack.push(children)
        break
      }

      case 'strong_close':
        stack.pop()
        break

      case 'em_open': {
        const children: InlineNode[] = []
        current.push({ type: 'emphasis', key: nextKey('em'), children })
        stack.push(children)
        break
      }

      case 'em_close':
        stack.pop()
        break

      case 's_open': {
        const children: InlineNode[] = []
        current.push({ type: 'strikethrough', key: nextKey('s'), children })
        stack.push(children)
        break
      }

      case 's_close':
        stack.pop()
        break

      case 'link_open': {
        const href = token.attrGet('href') ?? ''
        const title = token.attrGet('title')
        const children: InlineNode[] = []
        current.push({ type: 'link', key: nextKey('a'), href, title, children })
        stack.push(children)
        break
      }

      case 'link_close':
        stack.pop()
        break

      case 'image': {
        const src = token.attrGet('src') ?? ''
        const alt = token.content || (token.attrGet('alt') ?? '')
        const title = token.attrGet('title')
        current.push({ type: 'image', key: nextKey('img'), src, alt, title })
        break
      }

      default:
        // Unknown inline token — render as text if it has content
        if (token.content) {
          current.push({ type: 'text', key: nextKey('t'), content: token.content })
        }
        break
    }
  }

  return nodes
}

const detectTaskItem = (children: BlockNode[]): boolean | null => {
  if (children.length === 0) return null
  const first = children[0]
  if (first?.type !== 'paragraph' || first.children.length === 0) return null

  const firstInline = first.children[0]
  if (firstInline?.type !== 'text') return null

  const match = firstInline.content.match(/^\[([ xX])\]\s?/)
  if (!match) return null

  // Strip the checkbox markup from the text
  firstInline.content = firstInline.content.slice(match[0].length)

  // Remove the text node if it's now empty
  if (firstInline.content === '') {
    first.children.shift()
  }

  return match[1] !== ' '
}

const parseMarkdown = (markdown: string): BlockNode[] => {
  if (!markdown) return []

  resetKeys()
  const tokens = md.parse(markdown, {})
  const result: BlockNode[] = []
  const blockStack: BlockNode[][] = [result]

  // Table state
  let tableAlignments: Array<'left' | 'center' | 'right' | null> = []
  let tableHeader: TableRowNode | null = null
  let tableBody: TableRowNode[] = []
  let currentRow: TableCellNode[] = []
  let isHeaderRow = false
  let cellInline: InlineNode[] = []
  let inCell = false

  for (let i = 0; i < tokens.length; i++) {
    const token = tokens[i]!
    const current = blockStack[blockStack.length - 1]!

    switch (token.type) {
      // ── Paragraphs ──
      case 'paragraph_open':
        // Next token will be inline
        break

      case 'inline': {
        const inlineChildren = token.children ? parseInlineTokens(token.children) : []
        if (inCell) {
          cellInline = inlineChildren
        } else {
          current.push({ type: 'paragraph', key: nextKey('p'), children: inlineChildren })
        }
        break
      }

      case 'paragraph_close':
        break

      // ── Headings ──
      case 'heading_open': {
        const level = parseInt(token.tag.slice(1), 10) as HeadingNode['level']
        const inlineToken = tokens[i + 1]
        const children = inlineToken?.children ? parseInlineTokens(inlineToken.children) : []
        current.push({ type: 'heading', key: nextKey('h'), level, children })
        i += 1 // skip the inline token (heading_close is handled below)
        break
      }

      case 'heading_close':
        break

      // ── Code blocks ──
      case 'fence': {
        const language = token.info.trim() || null
        current.push({ type: 'code_block', key: nextKey('cb'), language, content: token.content })
        break
      }

      case 'code_block':
        current.push({ type: 'code_block', key: nextKey('cb'), language: null, content: token.content })
        break

      // ── Blockquotes ──
      case 'blockquote_open': {
        const children: BlockNode[] = []
        current.push({ type: 'blockquote', key: nextKey('bq'), children })
        blockStack.push(children)
        break
      }

      case 'blockquote_close':
        blockStack.pop()
        break

      // ── Lists ──
      case 'bullet_list_open': {
        const children: ListItemNode[] = []
        current.push({ type: 'list', key: nextKey('ul'), ordered: false, start: 1, children })
        blockStack.push(children as unknown as BlockNode[])
        break
      }

      case 'ordered_list_open': {
        const start = token.attrGet('start') ? parseInt(token.attrGet('start')!, 10) : 1
        const children: ListItemNode[] = []
        current.push({ type: 'list', key: nextKey('ol'), ordered: true, start, children })
        blockStack.push(children as unknown as BlockNode[])
        break
      }

      case 'bullet_list_close':
      case 'ordered_list_close':
        blockStack.pop()
        break

      case 'list_item_open': {
        const itemChildren: BlockNode[] = []
        const item: ListItemNode = {
          type: 'list_item',
          key: nextKey('li'),
          children: itemChildren,
          taskChecked: null,
        }
        current.push(item as unknown as BlockNode)
        blockStack.push(itemChildren)
        break
      }

      case 'list_item_close': {
        blockStack.pop()
        // Check for task list checkbox in the item we just closed
        // After pop, the parent list's children array is on top of the stack
        const parentList = blockStack[blockStack.length - 1]!
        const lastItem = parentList[parentList.length - 1] as ListItemNode | undefined
        if (lastItem?.type === 'list_item') {
          lastItem.taskChecked = detectTaskItem(lastItem.children)
        }
        break
      }

      // ── Tables ──
      case 'table_open':
        tableAlignments = []
        tableHeader = null
        tableBody = []
        break

      case 'table_close':
        if (tableHeader) {
          current.push({
            type: 'table',
            key: nextKey('tbl'),
            header: tableHeader,
            body: tableBody,
            alignments: tableAlignments,
          })
        }
        tableHeader = null
        tableBody = []
        tableAlignments = []
        break

      case 'thead_open':
        isHeaderRow = true
        break

      case 'thead_close':
        isHeaderRow = false
        break

      case 'tbody_open':
      case 'tbody_close':
        break

      case 'tr_open':
        currentRow = []
        break

      case 'tr_close': {
        const row: TableRowNode = { type: 'table_row', key: nextKey('tr'), cells: currentRow }
        if (isHeaderRow) {
          tableHeader = row
        } else {
          tableBody.push(row)
        }
        currentRow = []
        break
      }

      case 'th_open':
      case 'td_open': {
        inCell = true
        cellInline = []
        const style = token.attrGet('style') ?? ''
        if (isHeaderRow) {
          if (style.includes('center')) tableAlignments.push('center')
          else if (style.includes('right')) tableAlignments.push('right')
          else tableAlignments.push('left')
        }
        break
      }

      case 'th_close':
      case 'td_close': {
        const isHeader = token.type === 'th_close'
        currentRow.push({
          type: 'table_cell',
          key: nextKey('tc'),
          children: cellInline,
          isHeader,
        })
        inCell = false
        cellInline = []
        break
      }

      // ── Horizontal rule ──
      case 'hr':
        current.push({ type: 'hr', key: nextKey('hr') })
        break

      // ── HTML blocks ──
      case 'html_block':
        current.push({ type: 'html_block', key: nextKey('html'), content: token.content })
        break

      default:
        break
    }
  }

  return result
}

export { parseMarkdown, parseInlineTokens }
