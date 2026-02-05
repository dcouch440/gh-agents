import React from 'react';
import { Box, Text } from 'ink';
import { marked, type Token, type Tokens } from 'marked';

interface MarkdownProps {
  content: string;
}

function renderInline(tokens: Token[]): React.ReactNode[] {
  return tokens.map((token: Token, i: number) => {
    switch (token.type) {
      case 'strong':
        return (
          <Text key={i} bold>
            {renderInline(token.tokens ?? [])}
          </Text>
        );
      case 'em':
        return (
          <Text key={i} italic>
            {renderInline(token.tokens ?? [])}
          </Text>
        );
      case 'codespan':
        return (
          <Text key={i} color="yellow">
            {token.text}
          </Text>
        );
      case 'text':
        if ('tokens' in token && token.tokens) {
          return <Text key={i}>{renderInline(token.tokens)}</Text>;
        }
        return <Text key={i}>{token.text}</Text>;
      case 'br':
        return <Text key={i}>{'\n'}</Text>;
      default:
        if ('text' in token) {
          return <Text key={i}>{(token as { text: string }).text}</Text>;
        }
        if ('raw' in token) {
          return <Text key={i}>{(token as { raw: string }).raw}</Text>;
        }
        return null;
    }
  });
}

function renderBlock(token: Token, key: number): React.ReactNode {
  switch (token.type) {
    case 'paragraph':
      return <Text key={key}>{renderInline(token.tokens ?? [])}</Text>;

    case 'heading':
      return (
        <Text key={key} bold color="cyan">
          {renderInline(token.tokens ?? [])}
        </Text>
      );

    case 'code':
      return (
        <Box
          key={key}
          borderStyle="single"
          borderColor="gray"
          paddingX={1}
          marginTop={1}
        >
          <Text color="green">{token.text}</Text>
        </Box>
      );

    case 'list':
      return (
        <Box key={key} flexDirection="column">
          {token.items.map((item: Tokens.ListItem, i: number) => {
            const prefix = token.ordered ? `${(token.start || 1) + i}.` : '●';
            const inlineTokens =
              item.tokens?.[0] && 'tokens' in item.tokens[0]
                ? (item.tokens[0] as { tokens: Token[] }).tokens
                : [];
            return (
              <Text key={i}>
                {'  '}
                {prefix}{' '}
                {renderInline(inlineTokens)}
              </Text>
            );
          })}
        </Box>
      );

    case 'space':
      return <Text key={key}> </Text>;

    default:
      if ('tokens' in token && token.tokens) {
        return <Text key={key}>{renderInline(token.tokens)}</Text>;
      }
      if ('text' in token) {
        return <Text key={key}>{(token as { text: string }).text}</Text>;
      }
      return null;
  }
}

export function Markdown({ content }: MarkdownProps) {
  if (!content) return null;
  const tokens = marked.lexer(content);
  return (
    <Box flexDirection="column">
      {tokens.map((token, i) => renderBlock(token, i))}
    </Box>
  );
}
