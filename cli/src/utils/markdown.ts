import { marked } from 'marked';
import TerminalRenderer from 'marked-terminal';

marked.setOptions({
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  renderer: new TerminalRenderer() as any,
});

export function renderMarkdown(content: string): string {
  if (!content) return content;
  const rendered = marked.parse(content);
  if (typeof rendered !== 'string') return content;
  // Remove trailing newlines added by marked
  return rendered.replace(/\n+$/, '');
}
