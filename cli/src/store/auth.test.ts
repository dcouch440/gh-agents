import { describe, it, expect, beforeEach } from 'vitest';
import {
  getToken,
  setToken,
  clearToken,
  isTokenExpired,
  getServerUrl,
  setServerUrl,
} from './auth.js';

describe('auth store', () => {
  beforeEach(() => {
    clearToken();
    setServerUrl('http://localhost:3000');
  });

  describe('getToken / setToken', () => {
    it('returns null when no token is set', () => {
      expect(getToken()).toBeNull();
    });

    it('returns the token after setting it', () => {
      setToken('my-jwt-token', 3600);
      expect(getToken()).toBe('my-jwt-token');
    });
  });

  describe('clearToken', () => {
    it('clears a previously set token', () => {
      setToken('my-jwt-token', 3600);
      clearToken();
      expect(getToken()).toBeNull();
    });
  });

  describe('isTokenExpired', () => {
    it('returns true when no token is set', () => {
      expect(isTokenExpired()).toBe(true);
    });

    it('returns false for a token with future expiry', () => {
      setToken('my-jwt-token', 3600);
      expect(isTokenExpired()).toBe(false);
    });

    it('returns true for a token with past expiry', () => {
      setToken('my-jwt-token', -1);
      expect(isTokenExpired()).toBe(true);
    });
  });

  describe('getServerUrl / setServerUrl', () => {
    it('returns default server URL', () => {
      expect(getServerUrl()).toBe('http://localhost:3000');
    });

    it('returns custom server URL after setting it', () => {
      setServerUrl('http://localhost:8080');
      expect(getServerUrl()).toBe('http://localhost:8080');
    });
  });
});
