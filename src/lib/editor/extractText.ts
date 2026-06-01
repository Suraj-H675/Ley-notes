import type { JSONContent } from '@tiptap/react';

export function extractText(content: JSONContent | null): string {
  if (!content) return '';

  let text = '';

  if (content.text) {
    text += content.text;
  }

  if (content.content && Array.isArray(content.content)) {
    for (const node of content.content) {
      text += extractText(node);
      if (node.type === 'paragraph' || node.type === 'heading') {
        text += '\n';
      }
    }
  }

  return text.trim();
}

export function getTextPreview(content: JSONContent | null, maxLength = 150): string {
  const text = extractText(content);
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength).trim() + '...';
}

export function countWords(content: JSONContent | null): number {
  const text = extractText(content);
  if (!text.trim()) return 0;
  return text.trim().split(/\s+/).length;
}

export function countCharacters(content: JSONContent | null): number {
  return extractText(content).length;
}
