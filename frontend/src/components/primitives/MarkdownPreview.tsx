import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import remarkBreaks from 'remark-breaks'

type MarkdownPreviewProps = {
  content: string
  className?: string
}

const stripThinkingTags = (text: string): string =>
  text.replace(/<thinking>[\s\S]*?<\/thinking>/g, '')

function MarkdownPreview({ content, className }: MarkdownPreviewProps) {
  const cleaned = stripThinkingTags(content)

  return (
    <div
      className={`markdown-preview${className ? ` ${className}` : ''}`}
      style={{ overflow: 'auto', height: '100%' }}
    >
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkBreaks]}
        components={{
          code: ({ className: codeClassName, children, ...rest }) => {
            const isBlock = typeof codeClassName === 'string' && codeClassName.startsWith('language-')
            return (
              <code
                className={isBlock ? 'markdown-preview__code-block' : 'markdown-preview__code-inline'}
                {...rest}
              >
                {children}
              </code>
            )
          },
          pre: ({ children }) => (
            <pre className="markdown-preview__pre">{children}</pre>
          ),
          p: ({ children }) => (
            <p className="markdown-preview__p">{children}</p>
          ),
          ul: ({ children }) => (
            <ul className="markdown-preview__ul">{children}</ul>
          ),
          ol: ({ children }) => (
            <ol className="markdown-preview__ol">{children}</ol>
          ),
          table: ({ children }) => (
            <table className="markdown-preview__table">{children}</table>
          ),
          th: ({ children }) => (
            <th className="markdown-preview__th">{children}</th>
          ),
          td: ({ children }) => (
            <td className="markdown-preview__td">{children}</td>
          ),
        }}
      >
        {cleaned}
      </ReactMarkdown>
    </div>
  )
}

export { MarkdownPreview }
