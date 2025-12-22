import { useState } from 'react';
import { MarkdownContent } from './MarkdownContent';
import { Button, HStack, Box, Tabs } from '@chakra-ui/react';

interface MarkdownEditorProps {
  initialContent: string;
  onSave: (content: string) => Promise<void>;
  onCancel: () => void;
}

/**
 * Markdown editor with live preview and save functionality.
 * Allows editing insights with a side-by-side editor and preview.
 */
export function MarkdownEditor({ initialContent, onSave, onCancel }: MarkdownEditorProps) {
  const [content, setContent] = useState(initialContent);
  const [saving, setSaving] = useState(false);
  const [activeTab, setActiveTab] = useState<'edit' | 'preview'>('edit');

  const handleSave = async () => {
    setSaving(true);
    try {
      // Normalize content before saving: remove excessive newlines and trim
      const normalizedContent = content
        .replace(/\n{2,}/g, '\n')  // Replace 2+ newlines with 1
        .trim();
      await onSave(normalizedContent);
    } finally {
      setSaving(false);
    }
  };

  const hasChanges = content !== initialContent;

  return (
    <Box
      borderWidth="1px"
      borderColor="gray.200"
      borderRadius="md"
      overflow="hidden"
      bg="white"
    >
      {/* Tab Navigation */}
      <HStack
        px={4}
        py={2}
        borderBottomWidth="1px"
        borderBottomColor="gray.200"
        bg="gray.50"
        justify="space-between"
      >
        <HStack gap={2}>
          <button
            onClick={() => setActiveTab('edit')}
            style={{
              padding: '6px 12px',
              background: activeTab === 'edit' ? '#0078d4' : 'transparent',
              color: activeTab === 'edit' ? 'white' : '#666',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '13px',
              fontWeight: activeTab === 'edit' ? '500' : '400',
            }}
          >
            ✏️ Edit
          </button>
          <button
            onClick={() => setActiveTab('preview')}
            style={{
              padding: '6px 12px',
              background: activeTab === 'preview' ? '#0078d4' : 'transparent',
              color: activeTab === 'preview' ? 'white' : '#666',
              border: 'none',
              borderRadius: '4px',
              cursor: 'pointer',
              fontSize: '13px',
              fontWeight: activeTab === 'preview' ? '500' : '400',
            }}
          >
            👁️ Preview
          </button>
        </HStack>

        <HStack gap={2}>
          <Button
            size="sm"
            variant="outline"
            onClick={onCancel}
            px={3}
            py={1}
          >
            Cancel
          </Button>
          <Button
            size="sm"
            colorPalette="blue"
            onClick={handleSave}
            disabled={!hasChanges || saving}
            px={3}
            py={1}
          >
            {saving ? '💾 Saving...' : '💾 Save'}
          </Button>
        </HStack>
      </HStack>

      {/* Content Area */}
      <Box p={4}>
        {activeTab === 'edit' ? (
          <div>
            <textarea
              value={content}
              onChange={(e) => setContent(e.target.value)}
              placeholder="Enter markdown content..."
              style={{
                width: '100%',
                minHeight: '300px',
                padding: '12px',
                border: '1px solid #e0e0e0',
                borderRadius: '6px',
                fontSize: '14px',
                fontFamily: 'monospace',
                lineHeight: '1.6',
                resize: 'vertical',
              }}
            />
            <div
              style={{
                marginTop: '8px',
                fontSize: '12px',
                color: '#666',
              }}
            >
              Supports Markdown: **bold**, *italic*, lists, headings, code, etc.
            </div>
          </div>
        ) : (
          <div
            style={{
              minHeight: '300px',
              padding: '12px',
              border: '1px solid #e0e0e0',
              borderRadius: '6px',
              background: '#fafafa',
            }}
          >
            {content ? (
              <MarkdownContent content={content} />
            ) : (
              <div style={{ color: '#999', fontStyle: 'italic' }}>
                No content to preview
              </div>
            )}
          </div>
        )}
      </Box>
    </Box>
  );
}
