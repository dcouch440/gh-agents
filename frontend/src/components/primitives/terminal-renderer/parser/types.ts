// ── Inline Nodes ──────────────────────────────────────────────

type TextNode = { type: 'text'; key: string; content: string }
type StrongNode = { type: 'strong'; key: string; children: InlineNode[] }
type EmphasisNode = { type: 'emphasis'; key: string; children: InlineNode[] }
type StrikethroughNode = { type: 'strikethrough'; key: string; children: InlineNode[] }
type InlineCodeNode = { type: 'inline_code'; key: string; content: string }
type LinkNode = { type: 'link'; key: string; href: string; title: string | null; children: InlineNode[] }
type ImageNode = { type: 'image'; key: string; src: string; alt: string; title: string | null }
type SoftBreakNode = { type: 'softbreak'; key: string }
type HardBreakNode = { type: 'hardbreak'; key: string }

type InlineNode =
  | TextNode
  | StrongNode
  | EmphasisNode
  | StrikethroughNode
  | InlineCodeNode
  | LinkNode
  | ImageNode
  | SoftBreakNode
  | HardBreakNode

// ── Block Nodes ───────────────────────────────────────────────

type ParagraphNode = { type: 'paragraph'; key: string; children: InlineNode[] }
type HeadingNode = { type: 'heading'; key: string; level: 1 | 2 | 3 | 4 | 5 | 6; children: InlineNode[] }
type CodeBlockNode = { type: 'code_block'; key: string; language: string | null; content: string }
type BlockquoteNode = { type: 'blockquote'; key: string; children: BlockNode[] }
type ListNode = { type: 'list'; key: string; ordered: boolean; start: number; children: ListItemNode[] }
type ListItemNode = { type: 'list_item'; key: string; children: BlockNode[]; taskChecked: boolean | null }
type TableNode = {
  type: 'table'
  key: string
  header: TableRowNode
  body: TableRowNode[]
  alignments: Array<'left' | 'center' | 'right' | null>
}
type TableRowNode = { type: 'table_row'; key: string; cells: TableCellNode[] }
type TableCellNode = { type: 'table_cell'; key: string; children: InlineNode[]; isHeader: boolean }
type HorizontalRuleNode = { type: 'hr'; key: string }
type HtmlBlockNode = { type: 'html_block'; key: string; content: string }

type BlockNode =
  | ParagraphNode
  | HeadingNode
  | CodeBlockNode
  | BlockquoteNode
  | ListNode
  | ListItemNode
  | TableNode
  | HorizontalRuleNode
  | HtmlBlockNode

export type {
  InlineNode,
  TextNode,
  StrongNode,
  EmphasisNode,
  StrikethroughNode,
  InlineCodeNode,
  LinkNode,
  ImageNode,
  SoftBreakNode,
  HardBreakNode,
  BlockNode,
  ParagraphNode,
  HeadingNode,
  CodeBlockNode,
  BlockquoteNode,
  ListNode,
  ListItemNode,
  TableNode,
  TableRowNode,
  TableCellNode,
  HorizontalRuleNode,
  HtmlBlockNode,
}
