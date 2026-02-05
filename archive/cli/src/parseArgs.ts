import { createRequire } from 'module';

interface ParsedArgs {
  serverUrl?: string;
  version?: boolean;
  help?: boolean;
}

export function parseArgs(argv: string[]): ParsedArgs {
  const result: ParsedArgs = {};

  if (argv.includes('--version') || argv.includes('-v')) {
    result.version = true;
  }

  if (argv.includes('--help') || argv.includes('-h')) {
    result.help = true;
  }

  const idx = argv.indexOf('--server');
  if (idx !== -1 && idx + 1 < argv.length) {
    result.serverUrl = argv[idx + 1];
  }

  return result;
}

export function getVersion(): string {
  const require = createRequire(import.meta.url);
  const pkg = require('../package.json') as { version: string };
  return pkg.version;
}

export function printHelp(): void {
  console.log(`nexor - AI agent orchestration for GitHub workflows

Usage: nexor [options]

Options:
  --server <url>   Server URL (default: http://127.0.0.1:3000)
  -v, --version    Print version
  -h, --help       Show this help message`);
}
