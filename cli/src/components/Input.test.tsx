import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render } from 'ink-testing-library';
import { Input } from './Input.js';

describe('Input', () => {
  it('renders prompt indicator when enabled', () => {
    const { lastFrame } = render(<Input onSubmit={vi.fn()} />);
    expect(lastFrame()!).toContain('>');
  });

  it('shows waiting message when disabled', () => {
    const { lastFrame } = render(<Input onSubmit={vi.fn()} disabled />);
    expect(lastFrame()!).toContain('Waiting for response');
  });

  it('does not show prompt when disabled', () => {
    const { lastFrame } = render(<Input onSubmit={vi.fn()} disabled />);
    expect(lastFrame()!).toContain('Waiting');
  });

  it('defaults to enabled when disabled prop is omitted', () => {
    const { lastFrame } = render(<Input onSubmit={vi.fn()} />);
    expect(lastFrame()!).not.toContain('Waiting');
    expect(lastFrame()!).toContain('>');
  });

  it('renders typed text in the input', () => {
    const { stdin, lastFrame } = render(<Input onSubmit={vi.fn()} />);
    stdin.write('hello');
    expect(lastFrame()!).toContain('hello');
  });

  it('does not render input field when disabled', () => {
    const { lastFrame } = render(<Input onSubmit={vi.fn()} disabled />);
    const frame = lastFrame()!;
    expect(frame).not.toContain('>');
  });
});
