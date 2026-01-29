import ReactMarkdown from 'react-markdown';
import type { Components } from 'react-markdown';
import { CodeBlock } from '../CodeBlock';
import styles from './MarkdownContent.module.css';

interface MarkdownContentProps {
  content: string;
}

const components: Components = {
  code({ className, children, ...props }) {
    const match = /language-(\w+)/.exec(className || '');
    const language = match ? match[1] : '';
    const codeString = String(children).replace(/\n$/, '');

    // Block code has a language class from the parent pre > code
    if (language) {
      return <CodeBlock code={codeString} language={language} />;
    }

    return (
      <code className={styles.inlineCode} {...props}>
        {children}
      </code>
    );
  },
  pre({ children }) {
    // If children is a CodeBlock (from the code handler above), render directly
    return <>{children}</>;
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

export function MarkdownContent({ content }: MarkdownContentProps) {
  return (
    <div className={styles.markdown}>
      <ReactMarkdown components={components}>
        {content}
      </ReactMarkdown>
    </div>
  );
}
