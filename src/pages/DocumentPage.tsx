import { useCallback, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { useNode } from '@/hooks';
import { Button, Badge } from '@/components/ui';
import { ArrowLeft, MoreHorizontal } from 'lucide-react';
import { BlockEditor } from '@/components/editor';
import { extractText } from '@/lib/editor';
import { useWorkspaceStore } from '@/store';
import type { JSONContent } from '@tiptap/react';
import { formatRelative } from '@/lib/utils';

export function DocumentPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { node, updateNode } = useNode(id || null);
  const { addToRecentNodes, setLastOpenedNode } = useWorkspaceStore();

  useEffect(() => {
    if (id) {
      setLastOpenedNode(id);
      addToRecentNodes(id);
    }
  }, [id, setLastOpenedNode, addToRecentNodes]);

  const handleContentUpdate = useCallback(
    async (content: JSONContent) => {
      if (!id) return;
      const plainText = extractText(content);
      await updateNode({ content, plainText });
    },
    [id, updateNode]
  );

  if (!node) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center space-y-4">
          <h2 className="text-2xl font-bold">Page not found</h2>
          <p className="text-muted-foreground">
            This page doesn't exist or has been deleted.
          </p>
          <Button variant="outline" onClick={() => navigate('/')}>
            <ArrowLeft className="h-4 w-4 mr-2" />
            Go Home
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      <header className="flex items-center gap-4 border-b p-4">
        <Button
          variant="ghost"
          size="icon"
          onClick={() => navigate(-1)}
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <div className="flex-1">
          <h1 className="text-xl font-semibold">
            {node.emoji && <span className="mr-2">{node.emoji}</span>}
            {node.title || 'Untitled'}
          </h1>
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Badge variant="outline" className="text-xs">
              {node.type}
            </Badge>
            <span>·</span>
            <span>Updated {formatRelative(node.updatedAt)}</span>
          </div>
        </div>
        <Button variant="ghost" size="icon">
          <MoreHorizontal className="h-4 w-4" />
        </Button>
      </header>

      <main className="flex-1 overflow-auto">
        <div className="max-w-3xl mx-auto py-8">
          <BlockEditor
            content={node.content}
            onUpdate={handleContentUpdate}
            placeholder="Start typing, or press '/' for commands..."
          />
        </div>
      </main>
    </div>
  );
}
