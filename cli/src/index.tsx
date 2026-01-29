import { render } from 'ink';
import { App } from './App.js';

function parseArgs(argv: string[]): { serverUrl?: string } {
  const idx = argv.indexOf('--server');
  if (idx !== -1 && idx + 1 < argv.length) {
    return { serverUrl: argv[idx + 1] };
  }
  return {};
}

const { serverUrl } = parseArgs(process.argv.slice(2));

render(<App serverUrl={serverUrl} />);
