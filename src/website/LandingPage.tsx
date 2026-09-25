import {
  ArrowRight,
  BrainCircuit,
  FileSearch,
  GitBranch,
  History,
  LockKeyhole,
  Search,
  ShieldCheck,
  Terminal,
} from 'lucide-react';

const FEATURES = [
  {
    label: '01',
    title: 'Resume with context',
    body: 'Recover the smallest useful account of what was decided, attempted, verified, and left unresolved instead of rereading an entire project history.',
  },
  {
    label: '02',
    title: 'Know why it is here',
    body: 'Every durable memory keeps provenance so a human or agent can drill back to the session, revision, command, or source evidence behind it.',
  },
  {
    label: '03',
    title: 'Respect the present',
    body: 'Historical agent text stays historical. Current user intent, live project state, corrections, and Git applicability outrank stale memory.',
  },
  {
    label: '04',
    title: 'Keep it local',
    body: 'Project memory and indexes live on your machine. Context reaches a model only through an integration you configure and the sharing boundary you allow.',
  },
];

const PROMISES = [
  { icon: <LockKeyhole size={13} />, text: 'No account required' },
  { icon: <ShieldCheck size={13} />, text: 'Local project memory' },
  { icon: <GitBranch size={13} />, text: 'Revision-aware' },
];

const BRIEF = [
  ['Goal', 'Finish the provider hardening pass without repeating the failed shim approach.'],
  ['Decision', 'Use the native Git path; the wrapper must preserve stdout exactly.'],
  ['Dead end', 'Long polling loops caused transport timeouts and were abandoned.'],
  ['Verified', 'Linux, macOS, and Windows x64/ARM64 portability matrix passed.'],
  ['Open', 'Re-check the remaining private-state boundary before release.'],
];

export function LandingPage() {
  return (
    <div className="min-h-screen bg-[#101114] text-[#eae7df] selection:bg-[#c2b28f]/25" data-page="website">
      <header className="sticky top-0 z-30 border-b border-white/6 bg-[#101114]/88 backdrop-blur-md">
        <div className="mx-auto flex h-14 max-w-5xl items-center justify-between px-5">
          <a href="/" className="flex min-w-0 items-center gap-3 font-semibold tracking-tight">
            <span aria-hidden="true" className="flex size-7 shrink-0 items-center justify-center border border-white/12 bg-[#181a1d] text-xs font-bold">L</span>
            <span>Ley</span>
          </a>
          <nav aria-label="Landing page" className="absolute left-1/2 hidden -translate-x-1/2 items-center gap-6 text-sm text-[#b4b1a9] md:flex">
            <a href="#why" className="hover:text-white">Why Ley</a>
            <a href="#features" className="hover:text-white">How it helps</a>
            <a href="#desktop" className="hover:text-white">Desktop</a>
          </nav>
          <a href="https://github.com/Suraj-H675/Ley-notes" className="flex h-9 items-center gap-2 bg-[#c2b28f] px-3.5 text-sm font-semibold text-[#15161a] transition-colors hover:bg-[#d3c39d]">
            View source <ArrowRight size={14} />
          </a>
        </div>
      </header>

      <main>
        <section className="relative isolate overflow-hidden border-b border-white/5 px-5 pb-20 pt-20 md:pb-24 md:pt-28">
          <div aria-hidden="true" className="pointer-events-none absolute inset-x-0 top-0 -z-10 opacity-70 [background-image:linear-gradient(to_right,rgba(255,255,255,.04)_1px,transparent_1px)] [background-size:82px_100%]" />
          <div className="mx-auto grid max-w-5xl items-center gap-14 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)] lg:gap-16">
            <div>
              <p className="mb-5 max-w-md text-sm leading-6 text-[#a19e96]">Local continuity for coding agents.</p>
              <h1 className="max-w-xl font-serif text-5xl leading-[0.99] tracking-[-0.03em] md:text-[4.35rem]">Pick up where the last agent left off.</h1>
              <p className="mt-7 max-w-xl border-l border-white/10 pl-5 text-lg leading-8 text-[#aaa79f]">
                Ley keeps durable project memory small, cited, revision-aware, and under your control—so the next coding session can continue useful work without trusting stale summaries blindly.
              </p>
              <div className="mt-9 flex flex-col gap-3 sm:flex-row">
                <a href="#features" className="flex h-11 items-center justify-center gap-2 bg-[#c2b28f] px-5 text-sm font-semibold text-[#15161a] transition-colors hover:bg-[#d3c39d]">
                  See how Ley works <ArrowRight size={15} />
                </a>
                <a href="https://github.com/Suraj-H675/Ley-notes" className="flex h-11 items-center justify-center gap-2 border border-white/10 px-5 text-sm font-medium text-[#d8d5cd] transition-colors hover:border-white/18 hover:bg-white/4">
                  <Terminal size={15} /> Follow development
                </a>
              </div>
              <div className="mt-7 flex flex-wrap gap-x-5 gap-y-2 border-t border-white/6 pt-5 text-xs text-[#807d76]">
                {PROMISES.map(({ icon, text }) => (
                  <span key={text} className="flex items-center gap-1.5">{icon}{text}</span>
                ))}
              </div>
            </div>
            <BriefPreview />
          </div>
        </section>

        <section id="why" className="border-b border-white/5 bg-[#131417] px-5 py-20">
          <div className="mx-auto grid max-w-5xl gap-10 md:grid-cols-[0.72fr_1.28fr] md:items-start">
            <div>
              <p className="font-medium uppercase tracking-[0.15em] text-micro text-[#8a877f]">The problem</p>
              <h2 className="mt-4 font-serif text-3xl leading-[1.08] tracking-[-0.02em] md:text-4xl">Coding agents forget the hard-won parts.</h2>
            </div>
            <div className="grid gap-x-10 gap-y-6 text-base leading-7 text-[#a5a29a] sm:grid-cols-2">
              <p>A repository tells an agent what exists now. It usually does not tell it why a decision was made, which approach already failed, or what the previous session actually verified.</p>
              <p>Ley keeps that continuity separate from current source, then assembles only the historical context that is useful for the task in front of the agent.</p>
            </div>
          </div>
        </section>

        <section id="features" className="border-b border-white/5 px-5 py-20">
          <div className="mx-auto max-w-5xl">
            <div className="max-w-2xl">
              <p className="font-medium uppercase tracking-[0.15em] text-micro text-[#8a877f]">Continuity, not another transcript archive</p>
              <h2 className="mt-4 font-serif text-4xl leading-[1.05] tracking-[-0.025em]">Remember what changes the next decision.</h2>
              <p className="mt-4 text-[#b4b1a9]">Ley is being rebuilt around a smaller contract: brief the task, search historical memory, inspect exact evidence, and checkpoint meaningful state.</p>
            </div>
            <div role="list" className="mt-12 grid border-t border-white/7 md:grid-cols-2">
              {FEATURES.map((feature) => (
                <div key={feature.title} role="listitem" className="group relative border-b border-r border-white/7 p-6 last:border-b-0 md:p-8 md:[&:nth-child(2n)]:border-r-0">
                  <div className="flex min-h-7 items-start justify-between">
                    <h3 className="font-serif text-xl font-medium">{feature.title}</h3>
                    <span aria-hidden="true" className="font-mono text-xs tabular-nums text-[#a5a29a] transition-colors group-hover:text-[#c2b28f]">{feature.label}</span>
                  </div>
                  <p className="mt-3 max-w-md text-sm leading-6 text-[#b4b1a9]">{feature.body}</p>
                </div>
              ))}
            </div>
          </div>
        </section>

        <section id="desktop" className="px-5 py-20">
          <div className="mx-auto flex max-w-5xl flex-col gap-8 border-y border-white/8 bg-[#141518] p-8 md:flex-row md:items-center md:justify-between md:px-12 md:py-11">
            <div className="max-w-2xl">
              <p className="flex items-center gap-2 text-sm font-medium text-[#bfbcb4]"><BrainCircuit size={15} /> Native by design</p>
              <h2 className="mt-4 font-serif text-3xl leading-[1.06] tracking-[-0.022em]">One real app. One public website.</h2>
              <p className="mt-4 leading-7 text-[#b4b1a9]">The desktop app owns local project access, integrations, privacy controls, review, and evidence. This website only explains and showcases Ley; there is no reduced browser edition pretending to provide the same product.</p>
            </div>
            <div className="shrink-0 space-y-2 border-l-2 border-[#c2b28f]/70 pl-4 font-mono text-xs text-[#b4b1a9]">
              <p>desktop · local project access</p>
              <p>website · product + docs</p>
              <p className="text-[#7e7b74]">browser workspace · retired</p>
            </div>
          </div>
        </section>
      </main>

      <footer className="border-t border-white/6 px-5 py-7 text-sm text-[#a5a29a]">
        <div className="mx-auto flex max-w-5xl items-center justify-between"><span>Ley</span><span>Local-first by design.</span></div>
      </footer>
    </div>
  );
}

function BriefPreview() {
  return (
    <figure className="relative mx-auto w-full max-w-[610px] overflow-hidden border border-white/9 bg-[#17181b] shadow-[0_34px_84px_rgba(0,0,0,.38)]">
      <figcaption className="flex h-10 items-center justify-between border-b border-white/6 bg-[#141518] px-4 font-mono text-[11px] uppercase tracking-[0.09em] text-[#7d7a73]">
        <span className="text-[#b4b1a9]">Task brief</span>
        <span>5 records · bounded</span>
      </figcaption>
      <div className="p-5 sm:p-7">
        <div className="flex items-start justify-between gap-5 border-b border-white/6 pb-5">
          <div>
            <p className="text-xs uppercase tracking-[0.12em] text-[#8b887f]">Continue portability hardening</p>
            <p className="mt-2 max-w-md font-serif text-2xl leading-tight">Useful history, without pretending history is current source.</p>
          </div>
          <div className="flex size-10 shrink-0 items-center justify-center border border-white/8 bg-[#121316] text-[#c2b28f]"><History size={18} /></div>
        </div>
        <div className="divide-y divide-white/5">
          {BRIEF.map(([label, value]) => (
            <div key={label} className="grid gap-2 py-4 sm:grid-cols-[86px_minmax(0,1fr)]">
              <span className="font-mono text-[10px] uppercase tracking-[0.1em] text-[#8c887f]">{label}</span>
              <span className="text-[13px] leading-6 text-[#c5c1b8]">{value}</span>
            </div>
          ))}
        </div>
        <div className="mt-2 grid gap-2 border-t border-white/6 pt-4 text-[11px] text-[#918d84] sm:grid-cols-3">
          <span className="flex items-center gap-1.5"><GitBranch size={12} /> revision checked</span>
          <span className="flex items-center gap-1.5"><FileSearch size={12} /> cited evidence</span>
          <span className="flex items-center gap-1.5"><Search size={12} /> search for more</span>
        </div>
      </div>
    </figure>
  );
}
