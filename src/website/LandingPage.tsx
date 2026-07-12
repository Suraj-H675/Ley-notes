import {
  ArrowRight,
  Braces,
  Download,
  FileText,
  FolderOpen,
  GitBranch,
  Link2,
  LockKeyhole,
  Network,
  Search,
  Sparkles,
} from 'lucide-react';

const FEATURES = [
  { icon: <Link2 size={18} />, title: 'Links that think with you', body: 'Connect ideas with wiki-links. Ley derives backlinks, outgoing links, and unresolved mentions automatically.' },
  { icon: <Search size={18} />, title: 'Find it again', body: 'Search titles, content, aliases, tags, and paths from a keyboard-first quick switcher.' },
  { icon: <Network size={18} />, title: 'See the shape of knowledge', body: 'Move from a focused local neighborhood to the full vault graph without leaving your notes.' },
  { icon: <Braces size={18} />, title: 'Markdown without lock-in', body: 'Desktop vaults are ordinary folders and .md files. Your knowledge outlives Ley.' },
];

export function LandingPage() {
  return (
    <div className="min-h-full overflow-y-auto bg-[#0d0f12] text-[#f3f1eb] selection:bg-[#9b87f5]/30">
      <header className="sticky top-0 z-30 border-b border-white/8 bg-[#0d0f12]/85 backdrop-blur-xl">
        <div className="mx-auto flex h-16 max-w-6xl items-center justify-between px-5">
          <a href="/" className="flex items-center gap-2.5 font-semibold tracking-tight">
            <span className="flex size-8 items-center justify-center rounded-lg bg-[#9b87f5] text-[#111217]">L</span>
            <span>Ley</span>
          </a>
          <nav className="hidden items-center gap-7 text-sm text-white/60 md:flex">
            <a href="#why" className="hover:text-white">Why Ley</a>
            <a href="#features" className="hover:text-white">Features</a>
            <a href="#desktop" className="hover:text-white">Desktop</a>
          </nav>
          <a href="/app" className="flex items-center gap-2 rounded-lg border border-white/12 bg-white/6 px-3.5 py-2 text-sm font-medium hover:bg-white/10">
            Open web app <ArrowRight size={14} />
          </a>
        </div>
      </header>

      <main>
        <section className="relative isolate overflow-hidden px-5 pb-24 pt-24 md:pb-32 md:pt-32">
          <div className="absolute left-1/2 top-0 -z-10 h-[520px] w-[760px] -translate-x-1/2 rounded-full bg-[#7c67da]/12 blur-[120px]" />
          <div className="mx-auto grid max-w-6xl items-center gap-16 lg:grid-cols-[1.02fr_0.98fr]">
            <div>
              <div className="mb-6 inline-flex items-center gap-2 rounded-full border border-[#9b87f5]/25 bg-[#9b87f5]/8 px-3 py-1.5 text-xs font-medium text-[#b9aaf9]">
                <Sparkles size={13} /> Your notes. Their connections. Nothing else in the way.
              </div>
              <h1 className="max-w-3xl text-5xl font-semibold leading-[1.04] tracking-[-0.045em] md:text-7xl">
                A quiet place for a <span className="text-[#a997f3]">loud mind.</span>
              </h1>
              <p className="mt-7 max-w-xl text-lg leading-8 text-white/58">
                Ley is a local-first second brain for connected Markdown notes. Capture quickly, link naturally, and rediscover what you already know.
              </p>
              <div className="mt-9 flex flex-col gap-3 sm:flex-row">
                <a href="/app" className="flex items-center justify-center gap-2 rounded-lg bg-[#a997f3] px-5 py-3 text-sm font-semibold text-[#111217] hover:bg-[#b7a8f7]">
                  Start in your browser <ArrowRight size={15} />
                </a>
                <a href="#desktop" className="flex items-center justify-center gap-2 rounded-lg border border-white/12 bg-white/5 px-5 py-3 text-sm font-medium hover:bg-white/9">
                  <Download size={15} /> Get the desktop app
                </a>
              </div>
              <div className="mt-6 flex flex-wrap gap-x-5 gap-y-2 text-xs text-white/38">
                <span className="flex items-center gap-1.5"><LockKeyhole size={12} /> No account</span>
                <span className="flex items-center gap-1.5"><FolderOpen size={12} /> Filesystem vaults</span>
                <span className="flex items-center gap-1.5"><GitBranch size={12} /> Git-friendly</span>
              </div>
            </div>
            <KnowledgePreview />
          </div>
        </section>

        <section id="why" className="border-y border-white/8 bg-white/[0.018] px-5 py-20">
          <div className="mx-auto grid max-w-6xl gap-12 md:grid-cols-[0.8fr_1.2fr] md:items-start">
            <div>
              <div className="text-xs font-semibold uppercase tracking-[0.2em] text-[#9b87f5]">The premise</div>
              <h2 className="mt-4 text-3xl font-semibold tracking-[-0.03em]">The graph should emerge from thinking—not become another chore.</h2>
            </div>
            <div className="grid gap-5 text-base leading-7 text-white/55 sm:grid-cols-2">
              <p>Write in a familiar workspace. Ley quietly indexes links, headings, tags, and properties while you focus on the idea itself.</p>
              <p>Use the graph when it answers a question: what connects, what is isolated, and where an idea might lead next.</p>
            </div>
          </div>
        </section>

        <section id="features" className="px-5 py-24">
          <div className="mx-auto max-w-6xl">
            <div className="max-w-2xl">
              <div className="text-xs font-semibold uppercase tracking-[0.2em] text-[#9b87f5]">Built for recall</div>
              <h2 className="mt-4 text-4xl font-semibold tracking-[-0.035em]">Capture is only half the job.</h2>
              <p className="mt-4 text-white/50">Ley is designed around retrieving, connecting, and developing ideas over years—not collecting forgotten documents.</p>
            </div>
            <div className="mt-12 grid gap-px overflow-hidden rounded-xl border border-white/8 bg-white/8 md:grid-cols-2">
              {FEATURES.map((feature) => (
                <article key={feature.title} className="bg-[#111318] p-7 md:p-9">
                  <span className="flex size-9 items-center justify-center rounded-lg bg-[#9b87f5]/10 text-[#aa98f3]">{feature.icon}</span>
                  <h3 className="mt-5 font-semibold">{feature.title}</h3>
                  <p className="mt-2 max-w-md text-sm leading-6 text-white/48">{feature.body}</p>
                </article>
              ))}
            </div>
          </div>
        </section>

        <section id="desktop" className="px-5 pb-24">
          <div className="mx-auto flex max-w-6xl flex-col items-start justify-between gap-8 rounded-2xl border border-[#9b87f5]/18 bg-[#9b87f5]/7 p-8 md:flex-row md:items-center md:p-12">
            <div className="max-w-2xl">
              <div className="flex items-center gap-2 text-sm font-medium text-[#b5a5f5]"><FileText size={16} /> Desktop is where your files become the vault</div>
              <h2 className="mt-3 text-3xl font-semibold tracking-[-0.03em]">Keep every note in a folder you control.</h2>
              <p className="mt-3 leading-7 text-white/50">The native app reads and writes ordinary Markdown, supports external editors and Git, and keeps indexes disposable.</p>
            </div>
            <div className="shrink-0 rounded-lg border border-white/10 bg-black/20 px-5 py-3 text-sm text-white/55">Desktop builds coming from this repository</div>
          </div>
        </section>
      </main>

      <footer className="border-t border-white/8 px-5 py-8 text-sm text-white/35">
        <div className="mx-auto flex max-w-6xl items-center justify-between"><span>Ley</span><span>Local-first by design.</span></div>
      </footer>
    </div>
  );
}

function KnowledgePreview() {
  return (
    <div className="relative mx-auto aspect-[1.05] w-full max-w-[540px] rounded-2xl border border-white/10 bg-[#111318] p-4 shadow-[0_40px_100px_rgba(0,0,0,.45)]">
      <div className="flex items-center justify-between border-b border-white/8 px-2 pb-3 text-xs text-white/35"><span>Local graph · Learning</span><span>12 notes · 18 links</span></div>
      <svg viewBox="0 0 520 420" className="h-full w-full" aria-label="Connected notes preview">
        <g stroke="#6f668f" strokeOpacity=".42" strokeWidth="1">
          <path d="M258 202 L140 112 M258 202 L395 105 M258 202 L397 286 M258 202 L147 316 M140 112 L78 208 M140 112 L245 60 M395 105 L466 195 M397 286 L466 195 M397 286 L302 365 M147 316 L302 365 M147 316 L78 208" />
        </g>
        {[
          [258, 202, 12, '#a997f3'], [140, 112, 8, '#6ea8fe'], [395, 105, 7, '#79d0a5'], [397, 286, 8, '#e3ae67'], [147, 316, 7, '#d68bd2'], [78, 208, 5, '#6ea8fe'], [245, 60, 5, '#79d0a5'], [466, 195, 5, '#e3ae67'], [302, 365, 5, '#d68bd2'],
        ].map(([cx, cy, r, fill], index) => <circle key={index} cx={cx} cy={cy} r={r} fill={String(fill)} />)}
        <g fill="#e7e3dc" fontSize="11" fontFamily="system-ui">
          <text x="276" y="207">Learning</text><text x="99" y="94">Mental models</text><text x="365" y="87">Books</text><text x="414" y="310">Projects</text><text x="90" y="344">Daily notes</text>
        </g>
      </svg>
      <div className="absolute bottom-4 left-4 right-4 rounded-lg border border-white/8 bg-[#17191f]/95 px-4 py-3 backdrop-blur">
        <div className="text-xs font-medium">Learning</div><div className="mt-1 text-xs text-white/38">Connected to 4 notes · 2 backlinks · updated today</div>
      </div>
    </div>
  );
}
