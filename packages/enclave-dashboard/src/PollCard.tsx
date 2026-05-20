// Today's CRISP poll card — the most prominent surface when a poll is live.

import React, { useEffect, useRef, useState } from 'react'
import { STAGES, STAGE_TIMING, TODAYS_POLL } from './data'
import { LINKS } from './lib/links'

type Poll = typeof TODAYS_POLL

function StageBadge({ stageIdx }: { stageIdx: number }) {
  const s = STAGES[stageIdx]
  const variant = stageIdx >= 6 ? 'published' : stageIdx === 3 ? 'open' : 'working'
  return (
    <span className={`stage-badge stage-badge--${variant}`}>
      <span className='stage-badge__dot' />
      <span>{s.label}</span>
    </span>
  )
}

function EncryptedBallotGrid({ count, cap = 60 }: { count: number; cap?: number }) {
  const glyphs = Math.min(count, cap)
  const prevRef = useRef(glyphs)
  const [newSince, setNewSince] = useState(0)
  useEffect(() => {
    setNewSince(Math.max(0, glyphs - prevRef.current))
    prevRef.current = glyphs
  }, [glyphs])
  return (
    <div className='ballot-grid' aria-label={`${count} encrypted ballots received`}>
      <div className='ballot-grid__head'>
        <div className='ballot-grid__count' key={count}>
          {count.toLocaleString()}
        </div>
        <div className='ballot-grid__label'>
          encrypted ballots received
          <div className='ballot-grid__sub'>Each is sealed on the voter's device. None have been opened.</div>
        </div>
      </div>
      <div className='ballot-grid__grid' aria-hidden='true'>
        {Array.from({ length: glyphs }).map((_, i) => {
          const isNew = i >= glyphs - newSince
          return (
            <span key={i} className={`ballot-glyph ${isNew ? 'ballot-glyph--new' : ''}`} style={!isNew ? { animation: 'none' } : undefined}>
              <svg viewBox='0 0 14 10' width='14' height='10'>
                <rect x='0.5' y='0.5' width='13' height='9' rx='1.2' fill='none' stroke='currentColor' strokeWidth='0.8' />
                <path d='M0.7 1 L7 5.2 L13.3 1' fill='none' stroke='currentColor' strokeWidth='0.8' strokeLinecap='round' />
              </svg>
            </span>
          )
        })}
        {count > cap && <span className='ballot-glyph ballot-glyph--more'>+{(count - cap).toLocaleString()}</span>}
      </div>
    </div>
  )
}

function PrivacyExplainer() {
  const [open, setOpen] = useState(false)
  return (
    <div className={`privacy ${open ? 'privacy--open' : ''}`}>
      <button className='privacy__toggle' onClick={() => setOpen((o) => !o)} aria-expanded={open}>
        <span className='privacy__icon' aria-hidden='true'>
          <svg viewBox='0 0 16 16' width='14' height='14'>
            <path
              d='M8 1.5 L13 3.5 V8 C13 11 10.5 13.4 8 14.5 C5.5 13.4 3 11 3 8 V3.5 Z'
              fill='none'
              stroke='currentColor'
              strokeWidth='1.2'
              strokeLinejoin='round'
            />
          </svg>
        </span>
        <span>How this poll stayed private</span>
        <span className={`privacy__chev ${open ? 'privacy__chev--open' : ''}`} aria-hidden='true'>
          <svg viewBox='0 0 10 10' width='10' height='10'>
            <path d='M2 4 L5 7 L8 4' fill='none' stroke='currentColor' strokeWidth='1.4' strokeLinecap='round' strokeLinejoin='round' />
          </svg>
        </span>
      </button>
      {open && (
        <div className='privacy__body'>
          <ol className='privacy__steps'>
            <li>
              <span className='privacy__num'>1</span>
              <div>
                <b>Sealed on your device.</b> Each ballot is encrypted in your browser before it leaves you. The Interfold network only ever
                sees ciphertext.
              </div>
            </li>
            <li>
              <span className='privacy__num'>2</span>
              <div>
                <b>No single party holds the key.</b> A freshly-drawn committee generates a shared key collaboratively. No member can
                decrypt anything alone.
              </div>
            </li>
            <li>
              <span className='privacy__num'>3</span>
              <div>
                <b>The tally happens under encryption.</b> Votes are added together while still sealed. The aggregate — and only the
                aggregate — is ever decrypted.
              </div>
            </li>
            <li>
              <span className='privacy__num'>4</span>
              <div>
                <b>Verifiable on-chain.</b> Every stage transition is recorded publicly, so anyone can check that the process was followed.
                Individual ballots remain sealed forever.
              </div>
            </li>
          </ol>
          <a className='privacy__more' href={LINKS.architecture} target='_blank' rel='noreferrer'>
            Read the full privacy model →
          </a>
        </div>
      )}
    </div>
  )
}

function ResultPanel({ poll, variant }: { poll: Poll; variant: string }) {
  const totals = poll.result.totals
  const total = Object.values(totals).reduce((a, b) => a + b, 0)
  const items = poll.options.map((o) => ({
    ...o,
    count: totals[o.id] ?? 0,
    pct: total > 0 ? (totals[o.id] ?? 0) / total : 0,
  }))
  const winner = items.find((i) => i.id === poll.result.winner)!
  const showBars = variant === 'bars' || variant === 'all'
  const showSentence = variant === 'sentence' || variant === 'all'

  return (
    <div className='result'>
      <div className='result__head'>
        <div className='result__eyebrow'>Result · published on-chain</div>
        <h3 className='result__headline'>
          {winner.label}
          <span className='result__pct'>{Math.round(winner.pct * 100)}%</span>
        </h3>
      </div>
      {showSentence && (
        <p className='result__sentence'>
          Of {total.toLocaleString()} encrypted ballots, {winner.count.toLocaleString()} voted for{' '}
          <b>{winner.label.toLowerCase().replace(/^yes,\s*/, '')}</b>. Individual votes were never seen by anyone.
        </p>
      )}
      {showBars && (
        <ul className='result__bars'>
          {items.map((it) => (
            <li key={it.id} className={`result__bar ${it.id === poll.result.winner ? 'result__bar--win' : ''}`}>
              <div className='result__bar-row'>
                <span className='result__bar-label'>{it.label}</span>
                <span className='result__bar-pct'>
                  <span className='result__bar-count'>{it.count.toLocaleString()}</span>
                  <span className='result__bar-pct-num'>{(it.pct * 100).toFixed(0)}%</span>
                </span>
              </div>
              <div className='result__bar-track'>
                <div className='result__bar-fill' style={{ width: `${it.pct * 100}%` }} />
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

export default function PollCard({
  pollState,
  currentStageIdx,
  resultVariant,
  liveMode,
  onToggleLive,
  ballotCount,
  onNavigate,
  poll: pollProp,
}: {
  pollState: string
  currentStageIdx: number
  resultVariant: string
  liveMode?: boolean
  onToggleLive?: () => void
  ballotCount?: number
  onNavigate?: (view: string) => void
  poll?: Poll
}) {
  const poll = pollProp ?? TODAYS_POLL
  const effective = { ...poll, ballotCount: ballotCount ?? poll.ballotCount }
  const timing = STAGE_TIMING[STAGES[currentStageIdx].id]
  const isPublished = pollState === 'published'
  const isOpen = pollState === 'open'
  const isComputing = pollState === 'computing'

  return (
    <section className='poll-card' aria-label="Today's CRISP poll">
      <div className='poll-card__inner'>
        <header className='poll-card__head'>
          <div className='poll-card__eyebrow'>
            <span className='poll-card__kicker'>Today on CRISP</span>
            <span className='poll-card__sep'>·</span>
            <span className='poll-card__id'>{poll.id}</span>
          </div>
          {onToggleLive && (
            <button
              type='button'
              className={`live-toggle ${liveMode ? 'live-toggle--on' : ''}`}
              onClick={onToggleLive}
              aria-pressed={!!liveMode}
            >
              <span className='live-toggle__icon' aria-hidden='true'>
                {liveMode ? (
                  <svg viewBox='0 0 10 10' width='10' height='10'>
                    <rect x='2' y='1.5' width='2' height='7' fill='currentColor' />
                    <rect x='6' y='1.5' width='2' height='7' fill='currentColor' />
                  </svg>
                ) : (
                  <svg viewBox='0 0 10 10' width='10' height='10'>
                    <path d='M2 1 L9 5 L2 9 Z' fill='currentColor' />
                  </svg>
                )}
              </span>
              <span>{liveMode ? 'Pause demo' : 'Watch the lifecycle'}</span>
            </button>
          )}
          <StageBadge stageIdx={currentStageIdx} />
        </header>

        <h1 className='poll-card__question'>{poll.question}</h1>
        <p className='poll-card__context'>{poll.context}</p>

        <div className='poll-card__timing'>
          <div>
            <div className='poll-card__timing-label'>Time</div>
            <div className='poll-card__timing-value'>{timing.remaining}</div>
            <div className='poll-card__timing-sub'>{timing.sub}</div>
          </div>
          <div>
            <div className='poll-card__timing-label'>Opened</div>
            <div className='poll-card__timing-value mono'>{poll.opened}</div>
          </div>
          <div>
            <div className='poll-card__timing-label'>Closes</div>
            <div className='poll-card__timing-value mono'>{poll.closes}</div>
          </div>
        </div>

        {!isPublished && (
          <div className='poll-card__options'>
            <div className='poll-card__options-label'>Options on the ballot</div>
            <ul className='options'>
              {poll.options.map((o, i) => (
                <li key={o.id} className='option'>
                  <span className='option__index'>{String.fromCharCode(65 + i)}</span>
                  <span className='option__label'>{o.label}</span>
                </li>
              ))}
            </ul>
            <div className='poll-card__cta-row'>
              <span className='poll-card__cta-note'>
                {isOpen
                  ? "Voting is open. Ballots are encrypted on the voter's device and submitted to the network."
                  : isComputing
                    ? 'Voting has closed. The committee is now tallying the encrypted ballots.'
                    : 'Voting has not opened yet for this poll.'}
              </span>
              <a
                className='link-inline'
                href='#inspector'
                onClick={(e) => {
                  e.preventDefault()
                  onNavigate?.('inspector')
                }}
              >
                Inspect this E3
                <span className='btn__arrow' aria-hidden='true'>
                  →
                </span>
              </a>
            </div>
          </div>
        )}

        {isPublished && (
          <div className='poll-card__result'>
            <ResultPanel poll={poll} variant={resultVariant} />
            <PrivacyExplainer />
          </div>
        )}
      </div>

      <aside className='poll-card__side'>
        <EncryptedBallotGrid count={effective.ballotCount} />
      </aside>
    </section>
  )
}
