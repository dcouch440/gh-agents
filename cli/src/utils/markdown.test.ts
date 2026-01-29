import { describe, it, expect } from 'vitest';
import { renderMarkdown } from './markdown.js';

describe('renderMarkdown', () => {
  it('returns empty string for empty input', () => {
    expect(renderMarkdown('')).toBe('');
  });

  it('renders plain text without modification', () => {
    const result = renderMarkdown('Hello world');
    expect(result).toContain('Hello world');
  });

  it('renders headers with formatting', () => {
    const result = renderMarkdown('# Title');
    // marked-terminal renders headers with ANSI codes
    expect(result).toContain('Title');
  });

  it('renders code blocks', () => {
    const result = renderMarkdown('```\nconst x = 1;\n```');
    expect(result).toContain('const x = 1;');
  });

  it('renders inline code', () => {
    const result = renderMarkdown('Use `foo()` here');
    expect(result).toContain('foo()');
  });

  it('renders lists with bullets', () => {
    const result = renderMarkdown('- item one\n- item two');
    expect(result).toContain('item one');
    expect(result).toContain('item two');
  });

  it('renders bold text', () => {
    const result = renderMarkdown('This is **bold**');
    expect(result).toContain('bold');
  });

  it('does not end with a trailing newline', () => {
    const result = renderMarkdown('Hello');
    expect(result.endsWith('\n')).toBe(false);
  });
});
