import { Highlight, themes } from 'prism-react-renderer';
import { Copy, Check, ChevronDown, ChevronUp } from 'lucide-react';
import { useState, useEffect } from 'react';
import styles from './CodeBlock.module.css';

const COLLAPSE_THRESHOLD = 15;

interface CodeBlockProps {
  code: string;
  language: string;
}

export function CodeBlock({ code, language }: CodeBlockProps) {
  const [copied, setCopied] = useState(false);
  const [collapsed, setCollapsed] = useState(true);

  const lines = code.split('\n');
  const isLong = lines.length > COLLAPSE_THRESHOLD;

  const handleCopy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
  };

  useEffect(() => {
    if (copied) {
      const timer = setTimeout(() => setCopied(false), 2000);
      return () => clearTimeout(timer);
    }
  }, [copied]);

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <span className={styles.language}>{language}</span>
        <button
          onClick={handleCopy}
          className={styles.copyButton}
          aria-label="Copy code"
        >
          {copied ? <Check size={14} /> : <Copy size={14} />}
        </button>
      </div>
      <div className={`${styles.codeWrapper} ${isLong && collapsed ? styles.collapsed : ''}`}>
        <Highlight theme={themes.nightOwl} code={code} language={language}>
          {({ style, tokens, getLineProps, getTokenProps }) => (
            <pre className={styles.pre} style={style}>
              {tokens.map((line, i) => (
                <div key={i} {...getLineProps({ line })} className={styles.line}>
                  <span className={styles.lineNumber}>
                    {String(i + 1).padStart(2, ' ')}
                  </span>
                  {line.map((token, key) => (
                    <span key={key} {...getTokenProps({ token })} />
                  ))}
                </div>
              ))}
            </pre>
          )}
        </Highlight>
        {isLong && collapsed && <div className={styles.fadeOverlay} />}
      </div>
      {isLong && (
        <button className={styles.toggleBtn} onClick={() => setCollapsed(!collapsed)}>
          {collapsed ? (
            <>Show all {lines.length} lines <ChevronDown size={14} /></>
          ) : (
            <>Collapse <ChevronUp size={14} /></>
          )}
        </button>
      )}
    </div>
  );
}
