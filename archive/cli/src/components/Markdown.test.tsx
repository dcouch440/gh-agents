import React from 'react';
import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import { Markdown } from './Markdown.js';

describe('Markdown', () => {
  it('returns null for empty content', () => {
    const { lastFrame } = render(<Markdown content="" />);
    expect(lastFrame()).toBe('');
  });

  it('renders plain text', () => {
    const { lastFrame } = render(<Markdown content="Hello world" />);
    expect(lastFrame()!).toContain('Hello world');
  });

  it('renders bold text without asterisks', () => {
    const { lastFrame } = render(<Markdown content="This is **bold** text" />);
    const frame = lastFrame()!;
    expect(frame).toContain('bold');
    expect(frame).not.toContain('**');
  });

  it('renders italic text without asterisks', () => {
    const { lastFrame } = render(<Markdown content="This is *italic* text" />);
    const frame = lastFrame()!;
    expect(frame).toContain('italic');
    expect(frame).not.toContain('*italic*');
  });

  it('renders inline code', () => {
    const { lastFrame } = render(<Markdown content="Use `foo()` here" />);
    const frame = lastFrame()!;
    expect(frame).toContain('foo()');
    expect(frame).not.toContain('`');
  });

  it('renders code blocks', () => {
    const { lastFrame } = render(
      <Markdown content={'```\nconst x = 1;\n```'} />,
    );
    expect(lastFrame()!).toContain('const x = 1;');
  });

  it('renders unordered list items with bullets', () => {
    const { lastFrame } = render(
      <Markdown content={'- item one\n- item two'} />,
    );
    const frame = lastFrame()!;
    expect(frame).toContain('●');
    expect(frame).toContain('item one');
    expect(frame).toContain('item two');
  });

  it('renders ordered list items with numbers', () => {
    const { lastFrame } = render(
      <Markdown content={'1. first\n2. second'} />,
    );
    const frame = lastFrame()!;
    expect(frame).toContain('1.');
    expect(frame).toContain('first');
    expect(frame).toContain('second');
  });

  it('renders headings', () => {
    const { lastFrame } = render(<Markdown content="# Title" />);
    expect(lastFrame()!).toContain('Title');
  });

  it('renders multiple paragraphs', () => {
    const { lastFrame } = render(
      <Markdown content={'First paragraph\n\nSecond paragraph'} />,
    );
    const frame = lastFrame()!;
    expect(frame).toContain('First paragraph');
    expect(frame).toContain('Second paragraph');
  });
});
