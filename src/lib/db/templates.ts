import type { NodeTemplate } from '@/types';

export interface NodeTemplateDefinition {
  emoji: string;
  properties: Record<string, string>;
  defaultTitle: string;
}

export const NODE_TEMPLATES: Record<NodeTemplate, NodeTemplateDefinition> = {
  blank: {
    emoji: '📄',
    properties: {},
    defaultTitle: 'Untitled',
  },
  'book-note': {
    emoji: '📚',
    properties: {
      Author: '',
      Pages: '',
      Rating: '',
      Summary: '',
    },
    defaultTitle: 'Book Note',
  },
  'research-paper': {
    emoji: '🔬',
    properties: {
      Authors: '',
      DOI: '',
      Year: '',
      'Key Findings': '',
    },
    defaultTitle: 'Research Paper',
  },
  'meeting-note': {
    emoji: '📋',
    properties: {
      Date: '',
      Attendees: '',
      'Action Items': '',
    },
    defaultTitle: 'Meeting Notes',
  },
  person: {
    emoji: '👤',
    properties: {
      Role: '',
      Organization: '',
      Email: '',
    },
    defaultTitle: 'Person',
  },
  concept: {
    emoji: '💡',
    properties: {
      Domain: '',
      Definition: '',
      'Related Concepts': '',
    },
    defaultTitle: 'Concept',
  },
};
