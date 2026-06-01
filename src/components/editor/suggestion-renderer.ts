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
        { placement: 'bottom-start', middleware: [offset(8), flip(), shift({ padding: 8 })] }
      ).then(({ x, y }) => {
        if (popup) {
          popup.style.left = `${x}px`;
          popup.style.top = `${y}px`;
        }
      });
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

        options?.suggestion?.onStart?.(props);
      },

      onUpdate: (props: SuggestionRenderProps) => {
        component?.updateProps(props);

        if (props.clientRect) {
          updatePosition(props.clientRect);
        }

        options?.suggestion?.onUpdate?.(props);
      },

      onKeyDown: (props: SuggestionKeyDownProps) => {
        if (props.event.key === 'Escape') {
          popup?.remove();
          return true;
        }

        const ref = (component as any)?.ref;
        if (ref?.onKeyDown?.(props)) {
          return true;
        }

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
