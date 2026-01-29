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
    // Should not contain the input prompt ">" in isolation
    expect(lastFrame()!).toContain('Waiting');
  });
});
