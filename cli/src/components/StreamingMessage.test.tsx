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

  it('renders empty content with cursor', () => {
    const { lastFrame } = render(
      <StreamingMessage content="" done={false} />,
    );
    expect(lastFrame()!).toContain('█');
  });
});
