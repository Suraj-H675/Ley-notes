import { create } from 'zustand';
import { useEffect, useState } from 'react';
import { Check, AlertTriangle, Info, X } from 'lucide-react';
import { cn } from '@/lib/utils';

type ToastKind = 'success' | 'error' | 'info';
interface Toast {
  id: string;
  kind: ToastKind;
  message: string;
  description?: string;
}

interface ToastState {
  toasts: Toast[];
  push: (t: Omit<Toast, 'id'>) => string;
  dismiss: (id: string) => void;
}

let counter = 0;
const useToastStore = create<ToastState>((set) => ({
  toasts: [],
  push: (t) => {
    const id = `toast-${++counter}`;
    set((s) => ({ toasts: [...s.toasts, { ...t, id }] }));
    return id;
  },
  dismiss: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));

export function toast(message: string, opts?: { kind?: ToastKind; description?: string }) {
  return useToastStore.getState().push({
    kind: opts?.kind ?? 'info',
    message,
    description: opts?.description,
  });
}

export function Toaster() {
  const toasts = useToastStore((s) => s.toasts);
  return (
    <div className="pointer-events-none fixed bottom-4 right-4 z-[60] flex w-80 flex-col gap-2">
      {toasts.map((t) => (
        <ToastItem key={t.id} toast={t} />
      ))}
    </div>
  );
}

function ToastItem({ toast }: { toast: Toast }) {
  const [visible, setVisible] = useState(false);
  const dismiss = useToastStore((s) => s.dismiss);

  useEffect(() => {
    const t1 = setTimeout(() => setVisible(true), 10);
    const t2 = setTimeout(() => dismiss(toast.id), 3500);
    return () => {
      clearTimeout(t1);
      clearTimeout(t2);
    };
  }, [dismiss, toast.id]);

  const Icon =
    toast.kind === 'success' ? Check : toast.kind === 'error' ? AlertTriangle : Info;
  const accent =
    toast.kind === 'success'
      ? 'text-emerald-500'
      : toast.kind === 'error'
      ? 'text-rose-500'
      : 'text-foreground/80';

  return (
    <div
      role="status"
      className={cn(
        'pointer-events-auto flex items-start gap-2.5 rounded-md border border-border/80 bg-popover p-3 shadow-menu',
        'transition-all duration-200',
        visible ? 'translate-y-0 opacity-100' : 'translate-y-2 opacity-0'
      )}
    >
      <Icon className={cn('mt-0.5 h-4 w-4 flex-shrink-0', accent)} />
      <div className="min-w-0 flex-1">
        <div className="text-[13px] text-foreground/90">{toast.message}</div>
        {toast.description && (
          <div className="mt-0.5 text-[12px] text-muted-foreground/80">{toast.description}</div>
        )}
      </div>
      <button
        onClick={() => dismiss(toast.id)}
        aria-label="Dismiss"
        className="flex h-5 w-5 flex-shrink-0 items-center justify-center rounded text-muted-foreground/60 hover:bg-accent hover:text-foreground"
      >
        <X className="h-3 w-3" />
      </button>
    </div>
  );
}
