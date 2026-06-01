import { ReactRenderer } from '@tiptap/react';
import { computePosition, flip, shift, offset } from '@floating-ui/dom';

export interface SuggestionRendererOptions {
  suggestion?: {
    onStart?: (props: SuggestionRenderProps) => void;
    onUpdate?: (props: SuggestionRenderProps) => void;
    onKeyDown?: (props: SuggestionKeyDownProps) => boolean;
    onExit?: () => void;
  };
}

export interface SuggestionRenderProps {
  clientRect?: (() => DOMRect | null) | null;
  editor: any;
  items: any[];
  command: (props: any) => void;
}

export interface SuggestionKeyDownProps {
  event: KeyboardEvent;
}

export function createSuggestionRenderer(
  SuggestionComponent: React.ComponentType<any>,
  options?: SuggestionRendererOptions
) {
  return () => {
    let component: ReactRenderer | null = null;
    let popup: HTMLDivElement | null = null;

    const updatePosition = (clientRect: (() => DOMRect | null) | null | undefined) => {
      if (!popup || !clientRect) return;
      const rect = clientRect();
      if (!rect) return;

      computePosition(
        { getBoundingClientRect: () => rect } as Element,
        popup,
        { placement: 'bottom-start', middleware: [offset(6), flip(), shift({ padding: 8 })] }
      ).then(({ x, y }) => {
        if (popup) {
          popup.style.left = `${x}px`;
          popup.style.top = `${y}px`;
        }
      });
    };

    const animateIn = (el: HTMLElement) => {
      el.animate(
        [
          { opacity: 0, transform: 'translateY(-4px) scale(0.98)' },
          { opacity: 1, transform: 'translateY(0) scale(1)' },
        ],
        { duration: 120, easing: 'cubic-bezier(0.16, 1, 0.3, 1)', fill: 'forwards' }
      );
    };

    return {
      onStart: (props: SuggestionRenderProps) => {
        component = new ReactRenderer(SuggestionComponent, {
          props,
          editor: props.editor,
        });

        popup = document.createElement('div');
        popup.style.position = 'fixed';
        popup.style.zIndex = '9999';
        popup.style.left = '0';
        popup.style.top = '0';
        popup.appendChild(component.element);
        document.body.appendChild(popup);

        if (props.clientRect) {
          updatePosition(props.clientRect);
        }

        // Animate in once attached
        if (component.element instanceof HTMLElement) {
          animateIn(component.element);
        }

        options?.suggestion?.onStart?.(props);
      },

      onUpdate: (props: SuggestionRenderProps) => {
        component?.updateProps(props);
        if (props.clientRect) updatePosition(props.clientRect);
        options?.suggestion?.onUpdate?.(props);
      },

      onKeyDown: (props: SuggestionKeyDownProps) => {
        if (props.event.key === 'Escape') {
          popup?.remove();
          return true;
        }
        const ref = (component as any)?.ref;
        if (ref?.onKeyDown?.(props)) return true;
        return options?.suggestion?.onKeyDown?.(props) ?? false;
      },

      onExit: () => {
        popup?.remove();
        popup = null;
        component?.destroy();
        component = null;
        options?.suggestion?.onExit?.();
      },
    };
  };
}
