import { render } from 'ink';
import { App } from './App.js';
import { parseArgs, getVersion, printHelp } from './parseArgs.js';

const args = parseArgs(process.argv.slice(2));

if (args.version) {
  console.log(getVersion());
  process.exit(0);
}

if (args.help) {
  printHelp();
  process.exit(0);
}

const { waitUntilExit } = render(<App serverUrl={args.serverUrl} />);

process.on('SIGINT', () => {
  process.exit(0);
});

process.on('SIGTERM', () => {
  process.exit(0);
});

waitUntilExit().catch(() => {
  process.exit(1);
});
