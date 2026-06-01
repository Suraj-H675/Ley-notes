import { useCallback, useEffect } from 'react';
import { useEditor, EditorContent } from '@tiptap/react';
import StarterKit from '@tiptap/starter-kit';
import Placeholder from '@tiptap/extension-placeholder';
import CharacterCount from '@tiptap/extension-character-count';
import TaskList from '@tiptap/extension-task-list';
import TaskItem from '@tiptap/extension-task-item';
import Table from '@tiptap/extension-table';
import TableRow from '@tiptap/extension-table-row';
import TableHeader from '@tiptap/extension-table-header';
import TableCell from '@tiptap/extension-table-cell';
import Image from '@tiptap/extension-image';
import Link from '@tiptap/extension-link';
import Highlight from '@tiptap/extension-highlight';
import CodeBlockLowlight from '@tiptap/extension-code-block-lowlight';
import { common, createLowlight } from 'lowlight';
import { WikiLinkExtension, SlashCommandExtension } from './extensions';
import { EditorToolbar } from './EditorToolbar';
import { useEditorStore } from '@/store';
import { useAutoSave } from '@/hooks';
import type { JSONContent } from '@tiptap/react';

const lowlight = createLowlight(common);

interface BlockEditorProps {
  content: JSONContent | null;
  onUpdate: (content: JSONContent) => void;
  onSave?: () => void;
  editable?: boolean;
  placeholder?: string;
}

export function BlockEditor({
  content,
  onUpdate,
  onSave,
  editable = true,
  placeholder = "Type '/' for commands...",
}: BlockEditorProps) {
  const { setWordCount, setCharacterCount, setIsSaving } = useEditorStore();

  const handleSave = useCallback(
    async (newContent: JSONContent) => {
      setIsSaving(true);
      try {
        onUpdate(newContent);
        onSave?.();
      } finally {
        setIsSaving(false);
      }
    },
    [onUpdate, onSave, setIsSaving]
  );

  useAutoSave(content, {
    delay: 1000,
    onSave: handleSave,
    enabled: editable,
  });

  const editor = useEditor({
    extensions: [
      StarterKit.configure({
        codeBlock: false,
      }),
      Placeholder.configure({
        placeholder,
      }),
      CharacterCount.configure({
        limit: 50000,
      }),
      TaskList.configure({
        HTMLAttributes: {
          class: 'not-prose',
        },
      }),
      TaskItem.configure({
        nested: true,
      }),
      Table.configure({
        resizable: true,
      }),
      TableRow,
      TableHeader,
      TableCell,
      Image.configure({
        inline: true,
        allowBase64: true,
      }),
      Link.configure({
        openOnClick: false,
      }),
      Highlight.configure({
        multicolor: false,
      }),
      CodeBlockLowlight.configure({
        lowlight,
      }),
      WikiLinkExtension,
      SlashCommandExtension,
    ],
    content: content || '',
    editable,
    onUpdate: ({ editor }) => {
      const json = editor.getJSON();
      onUpdate(json);
      setWordCount(editor.storage.characterCount.words());
      setCharacterCount(editor.storage.characterCount.characters());
    },
    editorProps: {
      attributes: {
        class:
          'focus:outline-none min-h-[300px] py-1 text-[15px] leading-[1.7] text-foreground/90',
      },
    },
  });

  useEffect(() => {
    if (editor && content) {
      const currentContent = JSON.stringify(editor.getJSON());
      const newContent = JSON.stringify(content);
      if (currentContent !== newContent) {
        editor.commands.setContent(content);
      }
    }
  }, [editor, content]);

  return (
    <div className="relative">
      {editor && <EditorToolbar editor={editor} />}
      <EditorContent editor={editor} />
    </div>
  );
}
