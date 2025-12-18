import ReactMarkdown from 'react-markdown';

interface MarkdownContentProps {
  content: string;
}

/**
 * Renders Markdown content with proper styling.
 * Used for displaying AI-generated insights that contain Markdown formatting.
 */
export function MarkdownContent({ content }: MarkdownContentProps) {
  return (
    <ReactMarkdown
      components={{
        // Style for paragraphs
        p: ({ children }) => (
          <p style={{ margin: '0 0 8px 0', lineHeight: '1.6' }}>{children}</p>
        ),
        // Style for lists
        ul: ({ children }) => (
          <ul style={{ margin: '0', paddingLeft: '20px', lineHeight: '1.6' }}>
            {children}
          </ul>
        ),
        ol: ({ children }) => (
          <ol style={{ margin: '0', paddingLeft: '20px', lineHeight: '1.6' }}>
            {children}
          </ol>
        ),
        // Style for list items
        li: ({ children }) => (
          <li style={{ marginBottom: '4px' }}>{children}</li>
        ),
        // Style for bold text
        strong: ({ children }) => (
          <strong style={{ fontWeight: 600, color: '#2D3748' }}>{children}</strong>
        ),
        // Style for italic text
        em: ({ children }) => (
          <em style={{ fontStyle: 'italic', color: '#4A5568' }}>{children}</em>
        ),
        // Style for code (inline)
        code: ({ children }) => (
          <code
            style={{
              backgroundColor: '#F7FAFC',
              padding: '2px 6px',
              borderRadius: '3px',
              fontSize: '0.9em',
              fontFamily: 'monospace',
            }}
          >
            {children}
          </code>
        ),
        // Style for headings
        h1: ({ children }) => (
          <h1 style={{ fontSize: '1.5em', fontWeight: 600, margin: '0 0 12px 0' }}>
            {children}
          </h1>
        ),
        h2: ({ children }) => (
          <h2 style={{ fontSize: '1.3em', fontWeight: 600, margin: '0 0 10px 0' }}>
            {children}
          </h2>
        ),
        h3: ({ children }) => (
          <h3 style={{ fontSize: '1.1em', fontWeight: 600, margin: '0 0 8px 0' }}>
            {children}
          </h3>
        ),
      }}
    >
      {content}
    </ReactMarkdown>
  );
}
