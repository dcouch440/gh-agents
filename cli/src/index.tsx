import { render } from 'ink';
import { App } from './App.js';
import { parseArgs } from './parseArgs.js';

const { serverUrl } = parseArgs(process.argv.slice(2));

render(<App serverUrl={serverUrl} />);
