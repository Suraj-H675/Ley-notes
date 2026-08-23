import {
  ArrowRight,
  CalendarDays,
  Download,
  FileText,
  FolderOpen,
  GitBranch,
  LockKeyhole,
} from 'lucide-react';

const FEATURES = [
  {
    label: '01',
    title: 'Write first',
    body: 'Live Preview stays close to the page. Wiki links, headings, tasks, properties, and unresolved mentions become structure while you type.',
  },
  {
    label: '02',
    title: 'Find it later',
    body: 'Search across titles, content, aliases, tags, paths, and YAML properties from a quick switcher built for muscle memory.',
  },
  {
    label: '03',
    title: 'Follow the thought',
    body: 'Backlinks and a local graph appear when they answer something—not as another dashboard to maintain.',
  },
  {
    label: '04',
    title: 'Own the files',
    body: 'A desktop vault is just a folder of Markdown files. Git, grep, editors, backups, and scripts all keep working.',
  },
];

const PROMISES = [
  { icon: <LockKeyhole size={13} />, text: 'No account' },
  { icon: <FolderOpen size={13} />, text: 'Filesystem vaults' },
  { icon: <GitBranch size={13} />, text: 'Git-friendly' },
];

const VAULT_ITEMS = ['Learning', 'Mental models', 'Books', 'Projects', 'Daily notes'];
const BACKLINKS = ['Mental models', 'Reading queue', 'Project review'];
const TOKENS = ['[[Mental models]]', '#systems', '#reading'];

export function LandingPage() {
  return (
    <div className="min-h-screen bg-[#101114] text-[#eae7df] selection:bg-[#c2b28f]/25" data-page="website">
      <header className="sticky top-0 z-30 border-b border-white/6 bg-[#101114]/88 backdrop-blur-md">
        <div className="mx-auto flex h-14 max-w-5xl items-center justify-between px-5">
          <a href="/" className="flex min-w-0 items-center gap-3 font-semibold tracking-tight">
            <span aria-hidden="true" className="flex size-7 shrink-0 items-center justify-center border border-white/12 bg-[#181a1d] text-xs font-bold">L</span>
            <span>Ley</span>
          </a>
          <nav className="absolute left-1/2 hidden -translate-x-1/2 items-center gap-6 text-sm text-[#b4b1a9] md:flex">
            <a href="#why" className="hover:text-white">Why Ley</a>
            <a href="#features" className="hover:text-white">Features</a>
            <a href="#desktop" className="hover:text-white">Desktop</a>
          </nav>
          <a href="/app" className="flex h-9 items-center gap-2 bg-[#c2b28f] px-3.5 text-sm font-semibold text-[#15161a] transition-colors hover:bg-[#d3c39d]">
            Open web app <ArrowRight size={14} />
          </a>
        </div>
      </header>

      <main>
        <section className="relative isolate overflow-hidden border-b border-white/5 px-5 pb-20 pt-20 md:pb-24 md:pt-28">
          <div aria-hidden="true" className="pointer-events-none absolute inset-x-0 top-0 -z-10 opacity-70 [background-image:linear-gradient(to_right,rgba(255,255,255,.04)_1px,transparent_1px)] [background-size:82px_100%]" />
          <div className="mx-auto grid max-w-5xl items-center gap-14 lg:grid-cols-[minmax(0,0.92fr)_minmax(0,1.08fr)] lg:gap-16">
            <div>
              <p className="mb-5 max-w-sm text-sm leading-6 text-[#a19e96]">A local-first notebook for people whose ideas refuse to arrive one at a time.</p>
              <h1 className="max-w-xl font-serif text-5xl leading-[0.99] tracking-[-0.03em] md:text-[4.35rem]">Notes with a memory.</h1>
              <p className="mt-7 max-w-xl border-l border-white/10 pl-5 text-lg leading-8 text-[#aaa79f]">
                Write Markdown naturally. Let links, backlinks, and structure accumulate quietly around your thinking—without surrendering your files to a service.
              </p>
              <div className="mt-9 flex flex-col gap-3 sm:flex-row">
                <a href="/app" className="flex h-11 items-center justify-center gap-2 bg-[#c2b28f] px-5 text-sm font-semibold text-[#15161a] transition-colors hover:bg-[#d3c39d]">
                  Start in your browser <ArrowRight size={15} />
                </a>
                <a href="#desktop" className="flex h-11 items-center justify-center gap-2 border border-white/10 px-5 text-sm font-medium text-[#d8d5cd] transition-colors hover:border-white/18 hover:bg-white/4">
                  <Download size={15} /> Explore the desktop app
                </a>
              </div>
              <div className="mt-7 flex flex-wrap gap-x-5 gap-y-2 border-t border-white/6 pt-5 text-xs text-[#807d76]">
                {PROMISES.map(({ icon, text }) => (
                  <span key={text} className="flex items-center gap-1.5">{icon}{text}</span>
                ))}
              </div>
            </div>
            <KnowledgePreview />
          </div>
        </section>

        <section id="why" className="border-b border-white/5 bg-[#131417] px-5 py-20">
          <div className="mx-auto grid max-w-5xl gap-10 md:grid-cols-[0.72fr_1.28fr] md:items-start">
            <div>
              <p className="font-medium uppercase tracking-[0.15em] text-micro text-[#8a877f]">The premise</p>
              <h2 className="mt-4 font-serif text-3xl leading-[1.08] tracking-[-0.02em] md:text-4xl">The graph should emerge from thinking—not become another chore.</h2>
            </div>
            <div className="grid gap-x-10 gap-y-6 text-base leading-7 text-[#a5a29a] sm:grid-cols-2">
              <p>Write in a familiar workspace. Ley quietly indexes links, headings, tags, and properties while you focus on the idea itself.</p>
              <p>Use the graph when it answers a question: what connects, what is isolated, and where an idea might lead next.</p>
            </div>
          </div>
        </section>

        <section id="features" className="border-b border-white/5 px-5 py-20">
          <div className="mx-auto max-w-5xl">
            <div className="max-w-2xl">
              <p className="font-medium uppercase tracking-[0.15em] text-micro text-[#8a877f]">Built for recall</p>
              <h2 className="mt-4 font-serif text-4xl leading-[1.05] tracking-[-0.025em]">Capture is only half the job.</h2>
              <p className="mt-4 text-[#a5a29a]">Ley is designed around retrieving, connecting, and developing ideas over years—not collecting forgotten documents.</p>
            </div>
            <dl className="mt-12 grid border-t border-white/7 md:grid-cols-2">
              {FEATURES.map((feature) => (
                <article key={feature.title} className="group relative border-b border-r border-white/7 p-6 last:border-b-0 md:p-8 md:[&:nth-child(2n)]:border-r-0">
                  <div className="flex min-h-7 items-start justify-between">
                    <dt className="font-serif text-xl font-medium">{feature.title}</dt>
                    <span aria-hidden="true" className="font-mono text-xs tabular-nums text-[#6f6c65] transition-colors group-hover:text-[#c2b28f]">{feature.label}</span>
                  </div>
                  <dd className="mt-3 max-w-md text-sm leading-6 text-[#98958e]">{feature.body}</dd>
                </article>
              ))}
            </dl>
          </div>
        </section>

        <section id="desktop" className="px-5 py-20">
          <div className="mx-auto flex max-w-5xl flex-col gap-8 border-y border-white/8 bg-[#141518] p-8 md:flex-row md:items-center md:justify-between md:px-12 md:py-11">
            <div className="max-w-2xl">
              <p className="flex items-center gap-2 text-sm font-medium text-[#bfbcb4]"><FileText size={15} /> Desktop is where your files become the vault</p>
              <h2 className="mt-4 font-serif text-3xl leading-[1.06] tracking-[-0.022em]">Keep every note in a folder you control.</h2>
              <p className="mt-4 leading-7 text-[#a5a29a]">The native app reads and writes ordinary Markdown, supports external editors and Git, and keeps indexes disposable.</p>
            </div>
            <div className="shrink-0 border-l-2 border-[#c2b28f]/70 pl-4 font-mono text-sm text-[#b4b1a9]">npm run desktop:build</div>
          </div>
        </section>
      </main>

      <footer className="border-t border-white/6 px-5 py-7 text-sm text-[#75726b]">
        <div className="mx-auto flex max-w-5xl items-center justify-between"><span>Ley</span><span>Local-first by design.</span></div>
      </footer>
    </div>
  );
}

function KnowledgePreview() {
  return (
    <figure className="relative mx-auto w-full max-w-[590px] overflow-hidden border border-white/9 bg-[#17181b] shadow-[0_34px_84px_rgba(0,0,0,.38)]">
      <figcaption className="flex h-9 items-center justify-between border-b border-white/6 bg-[#141518] px-3 font-mono text-[11px] uppercase tracking-[0.09em] text-[#7d7a73]">
        <span>Learning.md</span>
      </figcaption>
      <div className="relative grid min-h-[430px] grid-cols-[190px_minmax(0,1fr)]">
        <aside className="hidden border-r border-white/6 bg-[#151619] p-3 sm:block">
          <p className="px-2 pb-2 text-[11px] font-medium uppercase tracking-[0.1em] text-[#6f6c65]">Vault</p>
          <nav className="space-y-0.5">
            {VAULT_ITEMS.map((name, index) => (
              <span key={name} className={`flex h-7 items-center truncate px-2 text-[12px] ${index === 0 ? 'bg-[#222427] text-[#dedbd3]' : 'text-[#8f8c85]'}`}>{name}</span>
            ))}
          </nav>
          <div className="mt-5 border-t border-white/5 pt-4">
            <p className="px-2 pb-1.5 text-[10px] uppercase tracking-[0.1em] text-[#64615b]">Backlinks</p>
            {BACKLINKS.map((item) => (
              <p key={item} className="truncate px-2 py-1 text-[11px] text-[#847f74]">{item}</p>
            ))}
          </div>
        </aside>
        <div className="min-w-0 p-5 sm:p-7">
          <div className="mb-5 border-b border-white/5 pb-4">
            <h3 className="font-serif text-3xl leading-tight text-[#eae7df]">Learning</h3>
            <p className="mt-1.5 font-mono text-[11px] text-[#6f6c65]">#systems · updated today</p>
          </div>
          <div className="space-y-3 text-[13px] leading-6 text-[#a8a59d]">
            <p>A note becomes useful when it can be found again. Ley derives those paths from ordinary writing instead of asking for metadata first.</p>
            <p>Links are visible but quiet. The graph is a tool for orientation, not a replacement for the page.</p>
          </div>
          <div className="mt-6 flex flex-wrap gap-1.5">
            {TOKENS.map((token) => (
              <code key={token} className="border border-white/7 bg-white/[0.035] px-2 py-1 font-mono text-[11px] text-[#cfc3a5]">{token}</code>
            ))}
          </div>
          <div className="mt-7 border border-white/6 bg-[#121316] p-3">
            <div className="flex items-center gap-2 text-[10px] uppercase tracking-[0.1em] text-[#6f6c65]"><CalendarDays size={11} /> Local graph · depth 2</div>
            <svg viewBox="0 0 340 120" className="mt-3 h-[92px]" role="img" aria-label="Small connected note graph">
              <g stroke="#63615b" strokeOpacity=".42" strokeWidth="1">
                <path d="M170 62 L92 32 M170 62 L246 33 M170 62 L108 94 M170 62 L236 93 M92 32 L43 63" />
              </g>
              {[
                [170, 62, 7, '#c2b28f'],
                [92, 32, 5, '#8f9a86'],
                [246, 33, 4, '#8f9a86'],
                [108, 94, 4, '#8f9a86'],
                [236, 93, 4, '#8f9a86'],
                [43, 63, 3, '#8f9a86'],
              ].map(([cx, cy, radius, fill], index) => (
                <circle key={index} cx={cx} cy={cy} r={radius} fill={String(fill)} />
              ))}
            </svg>
          </div>
        </div>
      </div>
      <div className="absolute bottom-4 right-4 hidden border border-white/8 bg-[#1b1c20]/95 px-3 py-2 backdrop-blur-sm sm:block">
        <p className="text-[12px] font-medium text-[#dedbd3]">Learning</p>
        <p className="mt-0.5 text-[11px] text-[#827f78]">Connected to 4 notes · 2 backlinks</p>
      </div>
    </figure>
  );
}
