import { forwardRef } from 'react';
import { cn } from '@/lib/utils';
import type { NodeTemplate } from '@/types';

interface Template {
  id: NodeTemplate;
  title: string;
  description: string;
  icon: string;
}

const TEMPLATES: Template[] = [
  {
    id: 'blank',
    title: 'Blank',
    description: 'Start with a clean slate',
    icon: '📄',
  },
  {
    id: 'book-note',
    title: 'Book Note',
    description: 'Capture insights from reading',
    icon: '📚',
  },
  {
    id: 'research-paper',
    title: 'Research Paper',
    description: 'Organize research findings',
    icon: '🔬',
  },
  {
    id: 'meeting-note',
    title: 'Meeting Note',
    description: 'Record meeting discussions',
    icon: '📝',
  },
  {
    id: 'person',
    title: 'Person',
    description: 'Profile for a person',
    icon: '👤',
  },
  {
    id: 'concept',
    title: 'Concept',
    description: 'Explore an idea',
    icon: '💡',
  },
];

interface TemplatePickerProps {
  onSelect: (template: NodeTemplate) => void;
  className?: string;
}

export const TemplatePicker = forwardRef<HTMLDivElement, TemplatePickerProps>(
  ({ onSelect, className }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          'bg-popover border rounded-md shadow-lg overflow-hidden max-h-[300px] overflow-y-auto',
          className
        )}
      >
        <div className="p-2">
          <p className="px-2 py-1 text-xs font-medium text-muted-foreground">
            Choose a template
          </p>
          {TEMPLATES.map((template) => (
            <button
              key={template.id}
              onClick={() => onSelect(template.id)}
              className="w-full flex items-center gap-3 px-2 py-2 text-sm text-left hover:bg-accent rounded-md transition-colors"
            >
              <span className="text-lg">{template.icon}</span>
              <div className="flex-1 min-w-0">
                <p className="font-medium">{template.title}</p>
                <p className="text-xs text-muted-foreground truncate">
                  {template.description}
                </p>
              </div>
            </button>
          ))}
        </div>
      </div>
    );
  }
);

TemplatePicker.displayName = 'TemplatePicker';
