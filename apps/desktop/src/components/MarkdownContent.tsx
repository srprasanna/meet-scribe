import ReactMarkdown from 'react-markdown';

interface MarkdownContentProps {
  content: string;
}

/**
 * Renders Markdown content with proper styling.
 * Used for displaying AI-generated insights that contain Markdown formatting.
 */
export function MarkdownContent({ content }: MarkdownContentProps) {
  // Preprocess content to normalize spacing:
  // 1. Remove all blank lines (multiple newlines -> single newline)
  // 2. Trim leading/trailing whitespace
  const normalizedContent = content
    .replace(/\n{2,}/g, '\n')  // Replace 2+ newlines with 1
    .trim();

  return (
    <ReactMarkdown
      components={{
        // Style for paragraphs
        p: ({ children, node }) => {
          // Paragraphs inside list items should have no margin
          // ReactMarkdown wraps list item text in <p> tags
          const parent = node?.parent;
          const isInListItem = parent?.type === 'listItem';

          return (
            <p style={{
              margin: isInListItem ? '0' : '0 0 6px 0',
              lineHeight: '1.5',
              whiteSpace: 'pre-wrap',
              wordWrap: 'break-word'
            }}>
              {children}
            </p>
          );
        },
        // Style for lists
        ul: ({ children, node }) => {
          // Check if this list is nested inside another list
          const parent = node?.parent;
          const isNested = parent?.type === 'listItem';

          return (
            <ul style={{
              margin: isNested ? '0' : '0 0 6px 0',
              paddingLeft: '28px',
              lineHeight: '1.5',
              listStyleType: 'disc',
              listStylePosition: 'outside'
            }}>
              {children}
            </ul>
          );
        },
        ol: ({ children, node }) => {
          // Check if this list is nested inside another list
          const parent = node?.parent;
          const isNested = parent?.type === 'listItem';

          return (
            <ol style={{
              margin: isNested ? '0' : '0 0 6px 0',
              paddingLeft: '28px',
              lineHeight: '1.5',
              listStyleType: 'decimal',
              listStylePosition: 'outside'
            }}>
              {children}
            </ol>
          );
        },
        // Style for list items
        li: ({ children }) => (
          <li style={{
            marginBottom: '2px',
            lineHeight: '1.5',
            whiteSpace: 'pre-wrap',
            wordWrap: 'break-word',
            display: 'list-item'
          }}>
            {children}
          </li>
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
          <h1 style={{ fontSize: '1.5em', fontWeight: 600, margin: '0 0 6px 0' }}>
            {children}
          </h1>
        ),
        h2: ({ children }) => (
          <h2 style={{ fontSize: '1.3em', fontWeight: 600, margin: '0 0 6px 0' }}>
            {children}
          </h2>
        ),
        h3: ({ children }) => (
          <h3 style={{ fontSize: '1.1em', fontWeight: 600, margin: '0 0 6px 0' }}>
            {children}
          </h3>
        ),
        h4: ({ children }) => (
          <h4 style={{ fontSize: '1em', fontWeight: 600, margin: '0 0 4px 0' }}>
            {children}
          </h4>
        ),
        h5: ({ children }) => (
          <h5 style={{ fontSize: '0.95em', fontWeight: 600, margin: '0 0 4px 0' }}>
            {children}
          </h5>
        ),
        h6: ({ children }) => (
          <h6 style={{ fontSize: '0.9em', fontWeight: 600, margin: '0 0 4px 0' }}>
            {children}
          </h6>
        ),
        // Style for blockquotes
        blockquote: ({ children }) => (
          <blockquote style={{
            margin: '4px 0 6px 0',
            paddingLeft: '12px',
            borderLeft: '3px solid #e0e0e0',
            color: '#666',
            fontStyle: 'italic'
          }}>
            {children}
          </blockquote>
        ),
        // Style for code blocks
        pre: ({ children }) => (
          <pre style={{
            margin: '4px 0 6px 0',
            padding: '8px',
            backgroundColor: '#f5f5f5',
            borderRadius: '4px',
            overflow: 'auto',
            fontSize: '0.9em'
          }}>
            {children}
          </pre>
        ),
        // Style for horizontal rules
        hr: () => (
          <hr style={{
            margin: '8px 0',
            border: 'none',
            borderTop: '1px solid #e0e0e0'
          }} />
        ),
      }}
    >
      {normalizedContent}
    </ReactMarkdown>
  );
}
