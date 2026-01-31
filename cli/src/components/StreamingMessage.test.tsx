import React from 'react';
import { describe, it, expect } from 'vitest';
import { render } from 'ink-testing-library';
import { StreamingMessage } from './StreamingMessage.js';

describe('StreamingMessage', () => {
  it('renders content text', () => {
    const { lastFrame } = render(
      <StreamingMessage content="Hello world" done={false} />,
    );
    expect(lastFrame()!).toContain('Hello world');
  });

  it('shows nexor label', () => {
    const { lastFrame } = render(
      <StreamingMessage content="test" done={false} />,
    );
    expect(lastFrame()!).toContain('nexor');
  });

  it('shows cursor when not done', () => {
    const { lastFrame } = render(
      <StreamingMessage content="partial" done={false} />,
    );
    expect(lastFrame()!).toContain('█');
  });

  it('hides cursor when done', () => {
    const { lastFrame } = render(
      <StreamingMessage content="complete" done={true} />,
    );
    expect(lastFrame()!).not.toContain('█');
  });

  it('renders separator line', () => {
    const { lastFrame } = render(
      <StreamingMessage content="" done={false} />,
    );
    expect(lastFrame()!).toContain('─');
  });

  it('shows empty content with cursor when not done', () => {
    const { lastFrame } = render(
      <StreamingMessage content="" done={false} />,
    );
    expect(lastFrame()!).toContain('█');
  });

  it('renders multiline content', () => {
    const { lastFrame } = render(
      <StreamingMessage content={'line one\nline two'} done={false} />,
    );
    const frame = lastFrame()!;
    expect(frame).toContain('line one');
    expect(frame).toContain('line two');
  });

  it('displays a timestamp', () => {
    const { lastFrame } = render(
      <StreamingMessage content="test" done={false} />,
    );
    const frame = lastFrame()!;
    expect(frame).toMatch(/\d{1,2}:\d{2}/);
  });

  it('renders long content', () => {
    const longContent = 'a'.repeat(200);
    const { lastFrame } = render(
      <StreamingMessage content={longContent} done={false} />,
    );
    const frame = lastFrame()!;
    const aCount = (frame.match(/a/g) || []).length;
    expect(aCount).toBeGreaterThanOrEqual(200);
  });

  it('shows content without cursor when done', () => {
    const { lastFrame } = render(
      <StreamingMessage content="Final answer" done={true} />,
    );
    const frame = lastFrame()!;
    expect(frame).toContain('Final answer');
    expect(frame).not.toContain('█');
  });

  it('renders markdown when done', () => {
    const { lastFrame } = render(
      <StreamingMessage content="This is **bold**" done={true} />,
    );
    const frame = lastFrame()!;
    expect(frame).toContain('bold');
    expect(frame).not.toContain('**');
  });

  it('renders separator with 60 dash characters', () => {
    const { lastFrame } = render(
      <StreamingMessage content="test" done={false} />,
    );
    expect(lastFrame()!).toContain('─'.repeat(60));
  });
});
