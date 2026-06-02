type JSONContent = {
  type?: string;
  text?: string;
  attrs?: Record<string, any>;
  marks?: Array<{ type: string; attrs?: Record<string, any> }>;
  content?: JSONContent[];
};

function escapeMarkdown(text: string): string {
  return text.replace(/([\\`*_{}\[\]()#+\-.!])/g, '\\$1');
}

function renderInline(node: JSONContent): string {
  if (!node) return '';
  if (node.type === 'text') {
    const text = node.text ?? '';
    const marks = node.marks ?? [];
    let out = escapeMarkdown(text);
    for (const mark of marks) {
      switch (mark.type) {
        case 'bold':
          out = `**${out}**`;
          break;
        case 'italic':
          out = `*${out}*`;
          break;
        case 'strike':
        case 'strikethrough':
          out = `~~${out}~~`;
          break;
        case 'code':
          out = '`' + text + '`'; // backticks wrap raw text, not escaped
          break;
        case 'link': {
          const href = mark.attrs?.href ?? '';
          out = `[${text}](${href})`;
          break;
        }
        case 'wikiLink': {
          const label = mark.attrs?.label ?? text;
          out = `[[${label}]]`;
          break;
        }
        case 'underline':
          // Markdown has no underline; use HTML as fallback
          out = `<u>${text}</u>`;
          break;
        case 'highlight':
          out = `==${text}==`;
          break;
        case 'subscript':
          out = `~${text}~`;
          break;
        case 'superscript':
          out = `^${text}^`;
          break;
        default:
          break;
      }
    }
    return out;
  }
  if (node.type === 'hardBreak') return '  \n';
  if (Array.isArray(node.content)) {
    return node.content.map(renderInline).join('');
  }
  return '';
}

function renderBlock(node: JSONContent): string {
  if (!node) return '';
  switch (node.type) {
    case 'paragraph': {
      const inner = (node.content ?? []).map(renderInline).join('');
      return inner;
    }
    case 'heading': {
      const level = Math.min(6, Math.max(1, node.attrs?.level ?? 1));
      const inner = (node.content ?? []).map(renderInline).join('');
      return `${'#'.repeat(level)} ${inner}`;
    }
    case 'codeBlock': {
      const lang = node.attrs?.language ?? '';
      const inner = (node.content ?? []).map((c) => c.text ?? '').join('');
      return `\`\`\`${lang}\n${inner}\n\`\`\``;
    }
    case 'bulletList': {
      return (node.content ?? [])
        .map((item) => {
          const inner = (item.content ?? []).map(renderBlock).join('\n');
          return `- ${inner}`;
        })
        .join('\n');
    }
    case 'orderedList': {
      return (node.content ?? [])
        .map((item, i) => {
          const inner = (item.content ?? []).map(renderBlock).join('\n');
          return `${i + 1}. ${inner}`;
        })
        .join('\n');
    }
    case 'listItem': {
      return (node.content ?? []).map(renderBlock).join('\n');
    }
    case 'taskList': {
      return (node.content ?? [])
        .map((item) => {
          const checked = item.attrs?.checked === true;
          const inner = (item.content ?? []).map(renderBlock).join('\n');
          return `${checked ? '- [x]' : '- [ ]'} ${inner}`;
        })
        .join('\n');
    }
    case 'taskItem': {
      const checked = node.attrs?.checked === true;
      const inner = (node.content ?? []).map(renderBlock).join('\n');
      return `${checked ? '- [x]' : '- [ ]'} ${inner}`;
    }
    case 'blockquote': {
      const inner = (node.content ?? []).map(renderBlock).join('\n');
      return inner
        .split('\n')
        .map((line) => (line ? `> ${line}` : '>'))
        .join('\n');
    }
    case 'horizontalRule':
      return '---';
    case 'image': {
      const src = node.attrs?.src ?? '';
      const alt = node.attrs?.alt ?? '';
      return `![${alt}](${src})`;
    }
    case 'hardBreak':
      return '  ';
    default: {
      // Fallback: render content recursively as block
      if (Array.isArray(node.content)) {
        return node.content.map(renderBlock).join('\n\n');
      }
      if (node.text) return node.text;
      return '';
    }
  }
}

export function tiptapJsonToMarkdown(json: JSONContent | null | undefined): string {
  if (!json || !Array.isArray(json.content)) return '';
  const blocks = json.content.map(renderBlock).filter((b) => b.length > 0);
  return blocks.join('\n\n');
}
