import { describe, it, expect } from 'vitest';
import { parseArgs } from './parseArgs.js';

describe('parseArgs', () => {
  it('returns serverUrl when --server flag is provided', () => {
    expect(parseArgs(['--server', 'http://localhost:8080'])).toEqual({
      serverUrl: 'http://localhost:8080',
    });
  });

  it('returns empty object when no flags provided', () => {
    expect(parseArgs([])).toEqual({});
  });

  it('returns empty object when --server has no value', () => {
    expect(parseArgs(['--server'])).toEqual({});
  });

  it('returns empty object for unrelated flags', () => {
    expect(parseArgs(['--other', 'value'])).toEqual({});
  });

  it('finds --server in the middle of argv', () => {
    expect(parseArgs(['--verbose', '--server', 'http://x:3000'])).toEqual({
      serverUrl: 'http://x:3000',
    });
  });

  it('uses first --server when duplicated', () => {
    expect(
      parseArgs(['--server', 'http://first', '--server', 'http://second']),
    ).toEqual({ serverUrl: 'http://first' });
  });

  it('accepts empty string as server URL value', () => {
    expect(parseArgs(['--server', ''])).toEqual({ serverUrl: '' });
  });

  it('parses --version flag', () => {
    expect(parseArgs(['--version'])).toEqual({ version: true });
  });

  it('parses -v shorthand', () => {
    expect(parseArgs(['-v'])).toEqual({ version: true });
  });

  it('parses --help flag', () => {
    expect(parseArgs(['--help'])).toEqual({ help: true });
  });

  it('parses -h shorthand', () => {
    expect(parseArgs(['-h'])).toEqual({ help: true });
  });

  it('combines --version and --server', () => {
    expect(parseArgs(['--version', '--server', 'http://x'])).toEqual({
      version: true,
      serverUrl: 'http://x',
    });
  });
});
