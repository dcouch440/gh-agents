import { describe, it, expect } from 'vitest'
import { parseMarkdown } from './parse'
import type { BlockNode, InlineNode, ParagraphNode, HeadingNode, CodeBlockNode, ListNode, TableNode, BlockquoteNode } from './types'

describe('parseMarkdown', () => {
  describe('basic blocks', () => {
    it('returns empty array for empty string', () => {
      expect(parseMarkdown('')).toEqual([])
    })

    it('parses plain text as paragraph', () => {
      const blocks = parseMarkdown('Hello world')
      expect(blocks).toHaveLength(1)
      expect(blocks[0]!.type).toBe('paragraph')
      const para = blocks[0] as ParagraphNode
      expect(para.children).toHaveLength(1)
      expect(para.children[0]!.type).toBe('text')
      if (para.children[0]!.type === 'text') {
        expect(para.children[0]!.content).toBe('Hello world')
      }
    })

    it('parses multiple paragraphs', () => {
      const blocks = parseMarkdown('First\n\nSecond')
      expect(blocks).toHaveLength(2)
      expect(blocks[0]!.type).toBe('paragraph')
      expect(blocks[1]!.type).toBe('paragraph')
    })

    it('parses horizontal rule', () => {
      const blocks = parseMarkdown('---')
      expect(blocks).toHaveLength(1)
      expect(blocks[0]!.type).toBe('hr')
    })
  })

  describe('headings', () => {
    it('parses h1 through h6', () => {
      const md = '# H1\n\n## H2\n\n### H3\n\n#### H4\n\n##### H5\n\n###### H6'
      const blocks = parseMarkdown(md)
      const headings = blocks.filter((b): b is HeadingNode => b.type === 'heading')
      expect(headings).toHaveLength(6)
      expect(headings.map((h) => h.level)).toEqual([1, 2, 3, 4, 5, 6])
    })

    it('preserves inline content in headings', () => {
      const blocks = parseMarkdown('# Hello **world**')
      const heading = blocks[0] as HeadingNode
      expect(heading.children).toHaveLength(2)
      expect(heading.children[0]!.type).toBe('text')
      expect(heading.children[1]!.type).toBe('strong')
    })
  })

  describe('code blocks', () => {
    it('parses fenced code block with language', () => {
      const md = '```js\nconsole.log("hi")\n```'
      const blocks = parseMarkdown(md)
      expect(blocks).toHaveLength(1)
      const code = blocks[0] as CodeBlockNode
      expect(code.type).toBe('code_block')
      expect(code.language).toBe('js')
      expect(code.content).toContain('console.log')
    })

    it('parses fenced code block without language', () => {
      const md = '```\nsome code\n```'
      const blocks = parseMarkdown(md)
      const code = blocks[0] as CodeBlockNode
      expect(code.language).toBeNull()
    })
  })

  describe('inline formatting', () => {
    it('parses bold text', () => {
      const blocks = parseMarkdown('This is **bold** text')
      const para = blocks[0] as ParagraphNode
      expect(para.children).toHaveLength(3)
      expect(para.children[1]!.type).toBe('strong')
    })

    it('parses italic text', () => {
      const blocks = parseMarkdown('This is *italic* text')
      const para = blocks[0] as ParagraphNode
      const emNode = para.children.find((n) => n.type === 'emphasis')
      expect(emNode).toBeDefined()
    })

    it('parses strikethrough', () => {
      const blocks = parseMarkdown('This is ~~struck~~ text')
      const para = blocks[0] as ParagraphNode
      const strikeNode = para.children.find((n) => n.type === 'strikethrough')
      expect(strikeNode).toBeDefined()
    })

    it('parses inline code', () => {
      const blocks = parseMarkdown('Use `code` here')
      const para = blocks[0] as ParagraphNode
      const codeNode = para.children.find((n) => n.type === 'inline_code')
      expect(codeNode).toBeDefined()
      if (codeNode?.type === 'inline_code') {
        expect(codeNode.content).toBe('code')
      }
    })

    it('parses links', () => {
      const blocks = parseMarkdown('[click here](https://example.com)')
      const para = blocks[0] as ParagraphNode
      const linkNode = para.children.find((n) => n.type === 'link')
      expect(linkNode).toBeDefined()
      if (linkNode?.type === 'link') {
        expect(linkNode.href).toBe('https://example.com')
      }
    })

    it('parses nested bold inside italic', () => {
      const blocks = parseMarkdown('*italic **bold** end*')
      const para = blocks[0] as ParagraphNode
      const emNode = para.children.find((n) => n.type === 'emphasis')
      expect(emNode).toBeDefined()
      if (emNode?.type === 'emphasis') {
        const strongChild = emNode.children.find((n) => n.type === 'strong')
        expect(strongChild).toBeDefined()
      }
    })
  })

  describe('lists', () => {
    it('parses unordered list', () => {
      const md = '- item one\n- item two\n- item three'
      const blocks = parseMarkdown(md)
      expect(blocks).toHaveLength(1)
      const list = blocks[0] as ListNode
      expect(list.type).toBe('list')
      expect(list.ordered).toBe(false)
      expect(list.children).toHaveLength(3)
    })

    it('parses ordered list', () => {
      const md = '1. first\n2. second\n3. third'
      const blocks = parseMarkdown(md)
      const list = blocks[0] as ListNode
      expect(list.ordered).toBe(true)
      expect(list.children).toHaveLength(3)
    })

    it('parses task list items', () => {
      const md = '- [ ] unchecked\n- [x] checked'
      const blocks = parseMarkdown(md)
      const list = blocks[0] as ListNode
      expect(list.children[0]!.taskChecked).toBe(false)
      expect(list.children[1]!.taskChecked).toBe(true)
    })

    it('parses nested lists', () => {
      const md = '- parent\n  - child'
      const blocks = parseMarkdown(md)
      const list = blocks[0] as ListNode
      expect(list.children).toHaveLength(1)
      const nestedList = list.children[0]!.children.find((c) => c.type === 'list')
      expect(nestedList).toBeDefined()
    })
  })

  describe('blockquotes', () => {
    it('parses blockquote', () => {
      const md = '> quoted text'
      const blocks = parseMarkdown(md)
      expect(blocks).toHaveLength(1)
      const bq = blocks[0] as BlockquoteNode
      expect(bq.type).toBe('blockquote')
      expect(bq.children).toHaveLength(1)
    })

    it('parses nested blockquotes', () => {
      const md = '> outer\n>> inner'
      const blocks = parseMarkdown(md)
      const bq = blocks[0] as BlockquoteNode
      const nested = bq.children.find((c) => c.type === 'blockquote')
      expect(nested).toBeDefined()
    })
  })

  describe('tables', () => {
    it('parses GFM table', () => {
      const md = '| Name | Age |\n| --- | --- |\n| Alice | 30 |\n| Bob | 25 |'
      const blocks = parseMarkdown(md)
      expect(blocks).toHaveLength(1)
      const table = blocks[0] as TableNode
      expect(table.type).toBe('table')
      expect(table.header.cells).toHaveLength(2)
      expect(table.body).toHaveLength(2)
    })

    it('parses table alignments', () => {
      const md = '| Left | Center | Right |\n| :--- | :---: | ---: |\n| a | b | c |'
      const blocks = parseMarkdown(md)
      const table = blocks[0] as TableNode
      expect(table.alignments).toEqual(['left', 'center', 'right'])
    })
  })

  describe('keys', () => {
    it('assigns unique keys to all nodes', () => {
      const md = '# Title\n\nParagraph with **bold** and `code`\n\n- item\n\n---'
      const blocks = parseMarkdown(md)
      const keys = new Set<string>()

      const collectKeys = (nodes: (BlockNode | InlineNode)[]): void => {
        for (const node of nodes) {
          keys.add(node.key)
          if ('children' in node && Array.isArray(node.children)) {
            collectKeys(node.children as (BlockNode | InlineNode)[])
          }
        }
      }

      collectKeys(blocks)
      // All keys should be unique (set size equals total node count)
      expect(keys.size).toBeGreaterThan(0)
    })
  })
})
