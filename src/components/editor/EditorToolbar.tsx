import { useCallback } from 'react';
import { BubbleMenu, type Editor } from '@tiptap/react';
import {
  Bold,
  Italic,
  Strikethrough,
  Code,
  Link as LinkIcon,
  Highlighter,
  Heading1,
  Heading2,
  List,
  ListOrdered,
  Quote,
  Code2,
} from 'lucide-react';
import { cn } from '@/lib/utils';

interface EditorToolbarProps {
  editor: Editor;
  className?: string;
}

export function EditorToolbar({ editor, className }: EditorToolbarProps) {
  const setLink = useCallback(() => {
    const previousUrl = editor.getAttributes('link').href;
    const url = window.prompt('URL', previousUrl);
    if (url === null) return;
    if (url === '') {
      editor.chain().focus().extendMarkRange('link').unsetLink().run();
      return;
    }
    editor.chain().focus().extendMarkRange('link').setLink({ href: url }).run();
  }, [editor]);

  if (!editor) return null;

  return (
    <BubbleMenu
      editor={editor}
      tippyOptions={{ duration: 100 }}
      className={cn(
        'flex items-center gap-0.5 rounded-md border border-border/80 bg-popover p-0.5 shadow-menu',
        className
      )}
    >
      <ToolButton
        active={editor.isActive('bold')}
        onClick={() => editor.chain().focus().toggleBold().run()}
        label="Bold"
      >
        <Bold className="h-3.5 w-3.5" />
      </ToolButton>
      <ToolButton
        active={editor.isActive('italic')}
        onClick={() => editor.chain().focus().toggleItalic().run()}
        label="Italic"
      >
        <Italic className="h-3.5 w-3.5" />
      </ToolButton>
      <ToolButton
        active={editor.isActive('strike')}
        onClick={() => editor.chain().focus().toggleStrike().run()}
        label="Strikethrough"
      >
        <Strikethrough className="h-3.5 w-3.5" />
      </ToolButton>
      <ToolButton
        active={editor.isActive('code')}
        onClick={() => editor.chain().focus().toggleCode().run()}
        label="Code"
      >
        <Code className="h-3.5 w-3.5" />
      </ToolButton>
      <ToolButton
        active={editor.isActive('highlight')}
        onClick={() => editor.chain().focus().toggleHighlight().run()}
        label="Highlight"
      >
        <Highlighter className="h-3.5 w-3.5" />
      </ToolButton>
      <Divider />
      <ToolButton active={editor.isActive('link')} onClick={setLink} label="Link">
        <LinkIcon className="h-3.5 w-3.5" />
      </ToolButton>
      <Divider />
      <ToolButton
        active={editor.isActive('heading', { level: 1 })}
        onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()}
        label="Heading 1"
      >
        <Heading1 className="h-3.5 w-3.5" />
      </ToolButton>
      <ToolButton
        active={editor.isActive('heading', { level: 2 })}
        onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
        label="Heading 2"
      >
        <Heading2 className="h-3.5 w-3.5" />
      </ToolButton>
      <ToolButton
        active={editor.isActive('bulletList')}
        onClick={() => editor.chain().focus().toggleBulletList().run()}
        label="Bullet list"
      >
        <List className="h-3.5 w-3.5" />
      </ToolButton>
      <ToolButton
        active={editor.isActive('orderedList')}
        onClick={() => editor.chain().focus().toggleOrderedList().run()}
        label="Numbered list"
      >
        <ListOrdered className="h-3.5 w-3.5" />
      </ToolButton>
      <ToolButton
        active={editor.isActive('blockquote')}
        onClick={() => editor.chain().focus().toggleBlockquote().run()}
        label="Quote"
      >
        <Quote className="h-3.5 w-3.5" />
      </ToolButton>
      <ToolButton
        active={editor.isActive('codeBlock')}
        onClick={() => editor.chain().focus().toggleCodeBlock().run()}
        label="Code block"
      >
        <Code2 className="h-3.5 w-3.5" />
      </ToolButton>
    </BubbleMenu>
  );
}

function ToolButton({
  active,
  onClick,
  label,
  children,
}: {
  active?: boolean;
  onClick: () => void;
  label: string;
  children: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      title={label}
      aria-label={label}
      className={cn(
        'flex h-7 w-7 items-center justify-center rounded text-muted-foreground/80 transition-colors',
        active ? 'bg-accent text-foreground' : 'hover:bg-accent/60 hover:text-foreground'
      )}
    >
      {children}
    </button>
  );
}

function Divider() {
  return <span className="mx-0.5 h-4 w-px bg-border/80" />;
}
