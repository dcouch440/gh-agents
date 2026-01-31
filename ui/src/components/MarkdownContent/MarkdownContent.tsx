import ReactMarkdown from 'react-markdown';
import type { Components } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import remarkBreaks from 'remark-breaks';
import { CodeBlock } from '../CodeBlock';
import styles from './MarkdownContent.module.css';

interface MarkdownContentProps {
  content: string;
}

const components: Components = {
  code({ className, children, node, ...props }) {
    const match = /language-(\w+)/.exec(className || '');
    const language = match ? match[1] : '';
    const codeString = String(children).replace(/\n$/, '');

    // Check if this is a block code (inside a <pre>) vs inline code
    const isBlock = node?.position && codeString.includes('\n');

    if (language) {
      return <CodeBlock code={codeString} language={language} />;
    }

    // Block code without a language (ASCII diagrams, plain code blocks)
    if (isBlock || className) {
      return (
        <pre className={styles.codeBlock}>
          <code>{children}</code>
        </pre>
      );
    }

    return (
      <code className={styles.inlineCode} {...props}>
        {children}
      </code>
    );
  },
  pre({ children }) {
    return <>{children}</>;
  },
  table({ children }) {
    return (
      <div className={styles.tableWrapper}>
        <table className={styles.table}>{children}</table>
      </div>
    );
  },
  th({ children }) {
    return <th className={styles.th}>{children}</th>;
  },
  td({ children }) {
    return <td className={styles.td}>{children}</td>;
  },
  p({ children }) {
    return <p className={styles.paragraph}>{children}</p>;
  },
  ul({ children }) {
    return <ul className={styles.list}>{children}</ul>;
  },
  ol({ children }) {
    return <ol className={styles.orderedList}>{children}</ol>;
  },
};

/** Strip <thinking>...</thinking> blocks from model output */
function stripThinking(text: string): string {
  return text.replace(/<thinking>[\s\S]*?<\/thinking>\s*/g, '');
}

export function MarkdownContent({ content }: MarkdownContentProps) {
  const cleaned = stripThinking(content);
  return (
    <div className={styles.markdown}>
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkBreaks]}
        components={components}
      >
        {cleaned}
      </ReactMarkdown>
    </div>
  );
}
