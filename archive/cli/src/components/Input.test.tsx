import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render } from 'ink-testing-library';
import { Input } from './Input.js';

describe('Input', () => {
  it('renders prompt indicator when enabled', () => {
    const { lastFrame } = render(<Input onSubmit={vi.fn()} />);
    expect(lastFrame()!).toContain('>');
  });

  it('shows sending message when sending', () => {
    const { lastFrame } = render(<Input onSubmit={vi.fn()} sending />);
    expect(lastFrame()!).toContain('Sending');
  });

  it('shows typing message when streaming', () => {
    const { lastFrame } = render(<Input onSubmit={vi.fn()} isStreaming />);
    expect(lastFrame()!).toContain('nexor is typing');
  });

  it('defaults to enabled when no state props provided', () => {
    const { lastFrame } = render(<Input onSubmit={vi.fn()} />);
    expect(lastFrame()!).not.toContain('Sending');
    expect(lastFrame()!).not.toContain('typing');
    expect(lastFrame()!).toContain('>');
  });

  it('renders typed text in the input', () => {
    const { stdin, lastFrame } = render(<Input onSubmit={vi.fn()} />);
    stdin.write('hello');
    expect(lastFrame()!).toContain('hello');
  });

  it('does not render input field when sending', () => {
    const { lastFrame } = render(<Input onSubmit={vi.fn()} sending />);
    const frame = lastFrame()!;
    expect(frame).not.toContain('>');
  });

  it('does not render input field when streaming', () => {
    const { lastFrame } = render(<Input onSubmit={vi.fn()} isStreaming />);
    const frame = lastFrame()!;
    expect(frame).not.toContain('>');
  });

  it('sending takes priority over streaming', () => {
    const { lastFrame } = render(
      <Input onSubmit={vi.fn()} sending isStreaming />,
    );
    expect(lastFrame()!).toContain('Sending');
    expect(lastFrame()!).not.toContain('typing');
  });
});
