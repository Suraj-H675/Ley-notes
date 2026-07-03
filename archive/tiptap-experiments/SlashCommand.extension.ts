// ARCHIVED — TipTap extension experiments, never wired up. See README.md.
import { Extension } from '@tiptap/core';
import { PluginKey } from '@tiptap/pm/state';
import Suggestion from '@tiptap/suggestion';
import { createSuggestionRenderer } from '../suggestion-renderer';
import { SlashCommandMenu } from '../SlashCommandMenu';

const SLASH_COMMANDS = [
  { id: 'heading1', title: 'Heading 1', description: 'Large section heading', category: 'inline', command: ({ editor, range }: any) => { editor.chain().focus().deleteRange(range).setNode('heading', { level: 1 }).run(); } },
  { id: 'heading2', title: 'Heading 2', description: 'Medium section heading', category: 'inline', command: ({ editor, range }: any) => { editor.chain().focus().deleteRange(range).setNode('heading', { level: 2 }).run(); } },
  { id: 'heading3', title: 'Heading 3', description: 'Small section heading', category: 'inline', command: ({ editor, range }: any) => { editor.chain().focus().deleteRange(range).setNode('heading', { level: 3 }).run(); } },
  { id: 'bulletList', title: 'Bullet List', description: 'Create a bullet list', category: 'lists', command: ({ editor, range }: any) => { editor.chain().focus().deleteRange(range).toggleBulletList().run(); } },
  { id: 'numberedList', title: 'Numbered List', description: 'Create a numbered list', category: 'lists', command: ({ editor, range }: any) => { editor.chain().focus().deleteRange(range).toggleOrderedList().run(); } },
  { id: 'taskList', title: 'Task List', description: 'Create a task list', category: 'lists', command: ({ editor, range }: any) => { editor.chain().focus().deleteRange(range).toggleTaskList().run(); } },
  { id: 'blockquote', title: 'Quote', description: 'Create a blockquote', category: 'blocks', command: ({ editor, range }: any) => { editor.chain().focus().deleteRange(range).toggleBlockquote().run(); } },
  { id: 'codeBlock', title: 'Code Block', description: 'Create a code block', category: 'blocks', command: ({ editor, range }: any) => { editor.chain().focus().deleteRange(range).setCodeBlock().run(); } },
  { id: 'horizontalRule', title: 'Divider', description: 'Insert a horizontal rule', category: 'blocks', command: ({ editor, range }: any) => { editor.chain().focus().deleteRange(range).setHorizontalRule().run(); } },
];

export const SlashCommandExtension = Extension.create({
  name: 'slashCommand',

  addOptions() {
    return {
      suggestion: {
        char: '/',
        pluginKey: new PluginKey('slashCommand'),
        startOfLine: false,
        items: ({ query }: { query: string }) => {
          return SLASH_COMMANDS.filter((cmd) =>
            cmd.title.toLowerCase().includes(query.toLowerCase()) ||
            cmd.description.toLowerCase().includes(query.toLowerCase())
          );
        },
        command: ({ editor, range, props }: any) => {
          props.command({ editor, range });
        },
        render: createSuggestionRenderer(SlashCommandMenu),
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
