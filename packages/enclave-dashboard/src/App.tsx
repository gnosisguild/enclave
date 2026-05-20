// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
// Main app shell + tweak wiring.

import { type ReactNode, useEffect, useMemo, useState } from 'react'
import { STAGES } from './data'
import PollCard from './PollCard'
import Timeline from './Timeline'
import History from './History'
import Pulse from './Pulse'
import Inspector from './Inspector'
import Loader from './Loader'
import { useAllE3s, useCrispPolls, useE3Details } from './lib/useE3s'
import { adaptHistoryEntries, adaptInspectorDetail, adaptInspectorE3List, adaptTodaysPoll } from './lib/adapt'
import { formatE3Id } from './lib/pollMeta'
import { LINKS, explorerAddress } from './lib/links'
import { CONTRACTS } from './lib/chain'
import type { E3FullDetails } from './lib/e3'
import { useTweaks } from './useTweaks'
import { TweaksPanel, TweakSection, TweakSelect, TweakRadio, TweakToggle } from './tweaks-panel'

function Header({ density, view, onNav }: { density: string; view: string; onNav: (id: string) => void }) {
  const link = (id: string, label: string) => (
    <a
      className={`site-nav__link ${view === id ? 'site-nav__link--on' : ''}`}
      href={`#${id}`}
      onClick={(e) => {
        e.preventDefault()
        onNav(id)
      }}
    >
      {label}
    </a>
  )
  return (
    <header className={`site-head site-head--${density}`}>
      <div className='site-head__inner'>
        <a
          className='wordmark'
          href='#'
          onClick={(e) => {
            e.preventDefault()
            onNav('inspector')
          }}
          aria-label='Interfold home'
        >
          <span className='wordmark__mark' aria-hidden='true'>
            <svg viewBox='0 0 22 22' width='22' height='22'>
              <path d='M2 19 L11 3 L20 19 Z' fill='none' stroke='currentColor' strokeWidth='1.5' strokeLinejoin='round' />
              <path d='M11 3 L11 19' stroke='currentColor' strokeWidth='1.5' strokeLinecap='round' />
              <path d='M11 11 L20 19' stroke='currentColor' strokeWidth='1.5' strokeLinecap='round' />
            </svg>
          </span>
          <span className='wordmark__text'>Interfold</span>
        </a>
        <nav className='site-nav' aria-label='Primary'>
          {link('inspector', 'E3 inspector')}
          {link('crisp', 'CRISP')}
        </nav>
      </div>
    </header>
  )
}

function Intro() {
  return (
    <section className='intro'>
      <div className='intro__eyebrow'>
        <span className='dot-live' /> Live · public poll
      </div>
      <h1 className='intro__title'>Watch an encrypted poll execute on the Interfold network.</h1>
      <p className='intro__lede'>
        CRISP is an example e3 running live. Ballots are encrypted on each voter's device, tallied without ever being decrypted, and only
        the final result is revealed. This page shows the lifecycle as it happens — and the archive of every poll that came before.
      </p>
    </section>
  )
}

function StatusNote({ children }: { children: ReactNode }) {
  return (
    <div className='emptystate'>
      <div className='emptystate__note'>
        <span className='emptystate__dot' aria-hidden='true' />
        <span>{children}</span>
      </div>
    </div>
  )
}

function SiteFooter() {
  return (
    <footer className='site-foot'>
      <div className='site-foot__inner'>
        <div className='site-foot__brand'>
          <div className='wordmark wordmark--foot'>
            <span className='wordmark__mark' aria-hidden='true'>
              <svg viewBox='0 0 22 22' width='20' height='20'>
                <path d='M2 19 L11 3 L20 19 Z' fill='none' stroke='currentColor' strokeWidth='1.5' strokeLinejoin='round' />
                <path d='M11 3 L11 19' stroke='currentColor' strokeWidth='1.5' strokeLinecap='round' />
                <path d='M11 11 L20 19' stroke='currentColor' strokeWidth='1.5' strokeLinecap='round' />
              </svg>
            </span>
            <span className='wordmark__text'>Interfold</span>
          </div>
          <p className='site-foot__tag'>
            Infrastructure for confidential coordination between independent parties. CRISP is one of the example applications running on
            the network.
          </p>
        </div>
        <div className='site-foot__cols'>
          <div>
            <div className='site-foot__col-head'>Learn</div>
            <a href={LINKS.docs} target='_blank' rel='noreferrer'>
              Documentation
            </a>
            <a href={LINKS.architecture} target='_blank' rel='noreferrer'>
              Architecture
            </a>
            <a href={LINKS.crisp} target='_blank' rel='noreferrer'>
              CRISP
            </a>
          </div>
          <div>
            <div className='site-foot__col-head'>Project</div>
            <a href={LINKS.repo} target='_blank' rel='noreferrer'>
              Open source
            </a>
            <a href={LINKS.blog} target='_blank' rel='noreferrer'>
              Blog
            </a>
            <a href={LINKS.site} target='_blank' rel='noreferrer'>
              Website
            </a>
          </div>
        </div>
      </div>
      <div className='site-foot__rule'>
        <span>© 2026 Interfold · Built in the open</span>
        <a className='mono' href={explorerAddress(CONTRACTS.Enclave)} target='_blank' rel='noreferrer'>
          Enclave on Sepolia ↗
        </a>
      </div>
    </footer>
  )
}

const TWEAK_DEFAULTS = {
  view: 'inspector',
  stageIdx: 3,
  pollState: 'open',
  density: 'comfortable',
  resultVariant: 'all',
  showPulse: true,
  accent: 'mint',
}

const ACCENT_PRESETS: Record<string, { bg: string; deep: string; soft: string; ink: string }> = {
  mint: { bg: '#e8faf0', deep: '#1f6b4a', soft: '#cdeede', ink: '#163d2c' },
  dusk: { bg: '#e6e8fa', deep: '#3a3f8a', soft: '#cdd2ee', ink: '#1f2347' },
  paper: { bg: '#f1ece2', deep: '#5a4a2a', soft: '#e3d9c2', ink: '#3a2f17' },
}

export default function App() {
  const [t, setTweak] = useTweaks(TWEAK_DEFAULTS)
  const setView = (v: string) => setTweak('view', v)

  const [, setNowTick] = useState(0)
  const [liveMode, setLiveMode] = useState(false)

  // ─── On-chain data (Sepolia) ──────────────────────────────────────────────
  // CRISP tab: only CRISP-program polls. Inspector tab: every E3 on the network.
  const crispPolls = useCrispPolls()
  const allE3s = useAllE3s()

  // The newest CRISP poll (first entry; lists are newest-first) is "today's poll".
  const todaysId = crispPolls.data && crispPolls.data.length > 0 ? crispPolls.data[0].id : null
  const todaysDetail = useE3Details(todaysId)

  // Inspector keeps its own selection — track which id is currently selected.
  const [inspectorIdStr, setInspectorIdStr] = useState<string | null>(null)
  const selectedInspectorId = useMemo(() => {
    if (!allE3s.data || allE3s.data.length === 0) return null
    if (inspectorIdStr) {
      const match = allE3s.data.find((e) => formatE3Id(e.id) === inspectorIdStr)
      if (match) return match.id
    }
    return allE3s.data[0].id
  }, [allE3s.data, inspectorIdStr])
  const inspectorDetail = useE3Details(selectedInspectorId)

  // Latest detail per E3 id, for history rows. Derived from the two detail
  // sources we actively poll (today's poll + the inspector selection) — no
  // effect needed, so no cascading renders.
  const detailsCache = useMemo(() => {
    const m = new Map<string, E3FullDetails>()
    if (todaysDetail.data) m.set(todaysDetail.data.id.toString(), todaysDetail.data)
    if (inspectorDetail.data) m.set(inspectorDetail.data.id.toString(), inspectorDetail.data)
    return m
  }, [todaysDetail.data, inspectorDetail.data])

  // CRISP tab state.
  const crispReady = crispPolls.status === 'ready'
  const polls = useMemo(() => crispPolls.data ?? [], [crispPolls.data])
  const hasPolls = polls.length > 0
  const livePoll = useMemo(() => (todaysDetail.data ? adaptTodaysPoll(todaysDetail.data) : null), [todaysDetail.data])
  const liveHistory = useMemo(() => (polls.length > 1 ? adaptHistoryEntries(polls.slice(1), detailsCache) : []), [polls, detailsCache])

  // Inspector tab state.
  const inspectorReady = allE3s.status === 'ready'
  const hasE3s = (allE3s.data?.length ?? 0) > 0
  const inspectorList = useMemo(() => adaptInspectorE3List(allE3s.data ?? []), [allE3s.data])
  const inspectorE3 = useMemo(() => adaptInspectorDetail(inspectorDetail.data), [inspectorDetail.data])

  // Sync poll stage to the live chain-derived stage whenever it changes (so
  // the timeline reflects reality, while still allowing manual overrides).
  const liveStageIdx = todaysDetail.data?.uiStageIdx
  useEffect(() => {
    if (liveStageIdx == null) return
    setTweak('stageIdx', liveStageIdx)
    setTweak('pollState', liveStageIdx >= 6 ? 'published' : liveStageIdx >= 4 ? 'computing' : 'open')
  }, [liveStageIdx, setTweak])

  const setStage = (i: number) => {
    setTweak('stageIdx', i)
    if (i >= 6) setTweak('pollState', 'published')
    else if (i >= 4) setTweak('pollState', 'computing')
    else setTweak('pollState', 'open')
  }

  useEffect(() => {
    const id = setInterval(() => setNowTick((n) => n + 1), 1000)
    return () => clearInterval(id)
  }, [])

  useEffect(() => {
    if (!liveMode) return undefined
    const program = [
      { state: 'open', stage: 0, hold: 2200 },
      { state: 'open', stage: 1, hold: 2200 },
      { state: 'open', stage: 2, hold: 2400 },
      { state: 'open', stage: 3, hold: 4600 },
      { state: 'computing', stage: 4, hold: 2800 },
      { state: 'computing', stage: 5, hold: 2400 },
      { state: 'published', stage: 6, hold: 4000 },
    ]
    let i = 0
    let cancelled = false
    let timer: ReturnType<typeof setTimeout> | null = null
    const advance = () => {
      if (cancelled) return
      const step = program[i]
      setTweak('pollState', step.state)
      setTweak('stageIdx', step.stage)
      i = (i + 1) % program.length
      timer = setTimeout(advance, step.hold)
    }
    advance()
    return () => {
      cancelled = true
      if (timer) clearTimeout(timer)
    }
  }, [liveMode, setTweak])

  const effectiveBallotCount = todaysDetail.data?.ballotCount ?? 0

  const reconciledStage = (() => {
    if (t.pollState === 'published') return 6
    if (t.pollState === 'computing') return Math.max(4, Math.min(5, t.stageIdx))
    if (t.pollState === 'none') return 6
    return Math.min(t.stageIdx, 3)
  })()

  useEffect(() => {
    const a = ACCENT_PRESETS[t.accent] ?? ACCENT_PRESETS.mint
    const root = document.documentElement
    root.style.setProperty('--accent-bg', a.bg)
    root.style.setProperty('--accent-deep', a.deep)
    root.style.setProperty('--accent-soft', a.soft)
    root.style.setProperty('--accent-ink', a.ink)
  }, [t.accent])

  const noActive = t.pollState === 'none'

  return (
    <div className={`page page--${t.density}`}>
      <Header density={t.density} view={t.view} onNav={setView} />
      {t.view === 'inspector' ? (
        <main className='main'>
          {allE3s.status === 'error' ? (
            <div className='inspector'>
              <StatusNote>Couldn't load E3s from Sepolia. Retrying automatically…</StatusNote>
            </div>
          ) : !inspectorReady ? (
            <div className='inspector'>
              <Loader label='Loading E3s' sub='Reading from Sepolia…' />
            </div>
          ) : !hasE3s ? (
            <div className='inspector'>
              <StatusNote>No E3s on the network yet. They will appear here once one is requested on-chain.</StatusNote>
            </div>
          ) : (
            <Inspector
              e3List={inspectorList}
              e3={inspectorE3}
              selectedId={selectedInspectorId ? formatE3Id(selectedInspectorId) : undefined}
              onSelect={(id) => setInspectorIdStr(id)}
              loading={inspectorDetail.status === 'loading'}
              error={inspectorDetail.status === 'error' ? inspectorDetail.error : null}
            />
          )}
        </main>
      ) : (
        <main className='main'>
          <Intro />

          {crispPolls.status === 'error' ? (
            <StatusNote>Couldn't load CRISP polls from Sepolia. Retrying automatically…</StatusNote>
          ) : !crispReady ? (
            <Loader label='Loading CRISP polls' sub='Reading from Sepolia…' />
          ) : !hasPolls ? (
            <StatusNote>
              No live CRISP polls right now. A new poll will appear here automatically when one is requested on-chain.
            </StatusNote>
          ) : todaysDetail.status === 'error' ? (
            <StatusNote>Couldn't load the latest poll details from Sepolia. Retrying automatically…</StatusNote>
          ) : !livePoll ? (
            <Loader label='Loading the latest poll' sub='Reading from Sepolia…' />
          ) : (
            <>
              <PollCard
                poll={livePoll}
                pollState={noActive ? 'published' : t.pollState}
                currentStageIdx={reconciledStage}
                resultVariant={t.resultVariant}
                liveMode={liveMode}
                onToggleLive={() => setLiveMode((v) => !v)}
                ballotCount={effectiveBallotCount}
                onNavigate={setView}
              />

              <Timeline
                stages={STAGES}
                currentStageIdx={reconciledStage}
                pollId={livePoll.id}
                density={t.density}
                onStageClick={setStage}
              />

              {liveHistory.length > 0 && <History entries={liveHistory} onNavigate={setView} />}
            </>
          )}
        </main>
      )}

      <Pulse
        data={{
          activeNow: todaysDetail.data && todaysDetail.data.uiStageIdx < 6 ? 1 : 0,
          ballots24h: todaysDetail.data?.ballotCount ?? 0,
          pollsAllTime: polls.length,
        }}
        show={t.showPulse}
      />
      <SiteFooter />

      <TweaksPanel title='Tweaks'>
        <TweakSection label='View' />
        <TweakRadio
          label='Tab'
          value={t.view}
          options={[
            { value: 'inspector', label: 'Inspector' },
            { value: 'crisp', label: 'CRISP' },
          ]}
          onChange={(v) => setTweak('view', v)}
        />

        <TweakSection label='Poll state' />
        <TweakSelect
          label='State'
          value={t.pollState}
          options={[
            { value: 'open', label: 'Open · accepting votes' },
            { value: 'computing', label: 'Computing · tallying' },
            { value: 'published', label: 'Published · result live' },
            { value: 'none', label: 'No active poll' },
          ]}
          onChange={(v) => setTweak('pollState', v)}
        />
        <TweakSelect
          label='Current stage'
          value={String(t.stageIdx)}
          options={STAGES.map((s, i) => ({
            value: String(i),
            label: `${i + 1}. ${s.label}`,
          }))}
          onChange={(v) => setTweak('stageIdx', Number(v))}
        />

        <TweakSection label='Presentation' />
        <TweakRadio label='Density' value={t.density} options={['compact', 'comfortable']} onChange={(v) => setTweak('density', v)} />
        <TweakSelect
          label='Result variant'
          value={t.resultVariant}
          options={[
            { value: 'all', label: 'Sentence + bars' },
            { value: 'bars', label: 'Bars only' },
            { value: 'sentence', label: 'Sentence only' },
          ]}
          onChange={(v) => setTweak('resultVariant', v)}
        />
        <TweakRadio label='Accent' value={t.accent} options={['mint', 'dusk', 'paper']} onChange={(v) => setTweak('accent', v)} />
        <TweakToggle label='Show network pulse' value={t.showPulse} onChange={(v) => setTweak('showPulse', v)} />
      </TweaksPanel>
    </div>
  )
}
