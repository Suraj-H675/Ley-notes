import { Extension } from '@tiptap/core';
import { PluginKey } from '@tiptap/pm/state';
import Suggestion from '@tiptap/suggestion';
import { createSuggestionRenderer } from '../suggestion-renderer';
import { WikiLinkSuggestionList } from '../WikiLinkSuggestion';

export const WikiLinkExtension = Extension.create({
  name: 'wikiLink',

  addOptions() {
    return {
      suggestion: {
        char: '[[',
        pluginKey: new PluginKey('wikiLink'),
        startOfLine: false,
        items: async ({ query }: { query: string }) => {
          const { db } = await import('@/lib/db');
          const nodes = await db.nodes
            .where('isArchived')
            .equals(0)
            .toArray();

          return nodes
            .filter((node) =>
              node.title.toLowerCase().includes(query.toLowerCase())
            )
            .slice(0, 10)
            .map((node) => ({
              id: node.id,
              title: node.title,
              type: node.type,
              emoji: node.emoji,
            }));
        },
        command: ({ editor, range, props }: any) => {
          editor.chain().focus().insertContentAt(range, [
            {
              type: 'text',
              marks: [{ type: 'wikiLink', attrs: { id: props.id, title: props.title } }],
              text: props.title,
            },
            { type: 'text', text: ' ' },
          ]).run();
        },
        render: createSuggestionRenderer(WikiLinkSuggestionList),
      },
    };
  },

  addProseMirrorPlugins() {
    return [
      Suggestion({
        editor: this.editor,
        ...this.options.suggestion,
      }),
    ];
  },
});
