import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { createPage } from '@/core/vault/pages';
import { resolveTitle } from '@/core/vault/page-index';
import { extractWikiLinks } from '@/core/parser/wiki-links';
import { useNavStore } from '@/shared/state/nav';

export function MarkdownReadingView({ content }: { content: string }) {
  async function follow(target: string) {
    const id = await resolveTitle(target) ?? (await createPage({ title: target })).id;
    const nav = useNavStore.getState();
    nav.openPage(id);
    nav.pushRecent(id);
  }

  return (
    <article className="markdown-reading mx-auto w-full max-w-[820px] px-10 pb-32 pt-8">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          a: ({ href, children }) => {
            if (href?.startsWith('ley:')) {
              const target = decodeURIComponent(href.slice(4).split('#')[0]);
              return <button type="button" className="wiki-reading-link" onClick={() => void follow(target)}>{children}</button>;
            }
            return <a href={href} target="_blank" rel="noreferrer">{children}</a>;
          },
          input: ({ type, checked, ...props }) => type === 'checkbox'
            ? <input type="checkbox" checked={checked} readOnly {...props} />
            : <input type={type} {...props} />,
        }}
      >
        {renderableMarkdown(content)}
      </ReactMarkdown>
    </article>
  );
}

function renderableMarkdown(content: string): string {
  let output = content;
  const links = extractWikiLinks(content).sort((left, right) => right.position - left.position);
  for (const link of links) {
    const label = link.alias ?? link.target;
    const anchor = link.blockId ? `#^${link.blockId}` : link.heading ? `#${link.heading}` : '';
    const replacement = link.isEmbed
      ? `> Embedded note: [${label}](ley:${encodeURIComponent(link.target)}${anchor})`
      : `[${label}](ley:${encodeURIComponent(link.target)}${anchor})`;
    output = `${output.slice(0, link.position)}${replacement}${output.slice(link.position + link.raw.length)}`;
  }
  return output;
}
