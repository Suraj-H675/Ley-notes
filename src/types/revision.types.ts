import type { JSONContent } from '@tiptap/react';

export interface Revision {
  id: string;
  nodeId: string;
  content: JSONContent;
  plainText: string;
  createdAt: number;
}

export interface CreateRevisionInput {
  nodeId: string;
  content: JSONContent;
  plainText: string;
}
