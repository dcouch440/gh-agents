import Conf from 'conf';

interface AuthConfig {
  token: string | null;
  tokenExpiry: number | null;
  serverUrl: string;
}

const config = new Conf<AuthConfig>({
  projectName: 'nexor-cli',
  defaults: {
    token: null,
    tokenExpiry: null,
    serverUrl: 'http://localhost:3000',
  },
});

export function getToken(): string | null {
  return config.get('token');
}

export function setToken(token: string, expiresIn: number): void {
  config.set('token', token);
  config.set('tokenExpiry', Date.now() + expiresIn * 1000);
}

export function clearToken(): void {
  config.set('token', null);
  config.set('tokenExpiry', null);
}

export function isTokenExpired(): boolean {
  const expiry = config.get('tokenExpiry');
  if (expiry === null) return true;
  return Date.now() >= expiry;
}

export function getServerUrl(): string {
  return config.get('serverUrl');
}

export function setServerUrl(url: string): void {
  config.set('serverUrl', url);
}
