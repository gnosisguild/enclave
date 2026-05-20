// Poll history archive — past completed polls. Rows expand to show detail.

import React, { useState } from 'react'
import { STAGES } from './data'

type Entry = {
  id: string
  question: string
  closed: string
  duration: string
  ballotCount: number
  result: string
}

const HIST_FILTERS = [
  { id: 'all', label: 'All', test: () => true },
  { id: 'appr', label: 'Approved', test: (e: Entry) => /Approved|Adopted/i.test(e.result) },
  { id: 'decl', label: 'Declined', test: (e: Entry) => /Declined/i.test(e.result) },
  { id: '2026', label: '2026', test: (e: Entry) => /2026/.test(e.closed) },
]

function pctFromResultStr(s: string) {
  const m = s.match(/(\d+)%/)
  return m ? Number(m[1]) : 50
}

function MiniTimeline({ stages }: { stages: typeof STAGES }) {
  return (
    <div className='mini-timeline' aria-hidden='true'>
      {stages.map((s, i) => (
        <React.Fragment key={s.id}>
          <span className='mini-timeline__dot' title={s.label} />
          {i < stages.length - 1 && <span className='mini-timeline__rule' />}
        </React.Fragment>
      ))}
      <span className='mini-timeline__label'>All seven stages completed</span>
    </div>
  )
}

function HistoryDetail({ entry, onNavigate }: { entry: Entry; onNavigate?: (view: string) => void }) {
  const winnerPct = pctFromResultStr(entry.result)
  const isApproved = !/Declined/i.test(entry.result)
  const otherPct = 100 - winnerPct
  const absPct = Math.min(8, Math.max(2, Math.round(otherPct * 0.15)))
  const losePct = otherPct - absPct
  const winnerLabel = isApproved ? 'Yes / Approve' : 'No / Decline'
  const loseLabel = isApproved ? 'No / Decline' : 'Yes / Approve'

  return (
    <div className='hist-detail'>
      <div className='hist-detail__grid'>
        <div className='hist-detail__col'>
          <div className='hist-detail__head'>Final tally</div>
          <ul className='result__bars hist-detail__bars'>
            <li className='result__bar result__bar--win'>
              <div className='result__bar-row'>
                <span className='result__bar-label'>{winnerLabel}</span>
                <span className='result__bar-pct'>
                  <span className='result__bar-pct-num'>{winnerPct}%</span>
                </span>
              </div>
              <div className='result__bar-track'>
                <div className='result__bar-fill' style={{ width: `${winnerPct}%` }} />
              </div>
            </li>
            <li className='result__bar'>
              <div className='result__bar-row'>
                <span className='result__bar-label'>{loseLabel}</span>
                <span className='result__bar-pct'>
                  <span className='result__bar-pct-num'>{losePct}%</span>
                </span>
              </div>
              <div className='result__bar-track'>
                <div className='result__bar-fill' style={{ width: `${losePct}%` }} />
              </div>
            </li>
            <li className='result__bar'>
              <div className='result__bar-row'>
                <span className='result__bar-label'>Abstain</span>
                <span className='result__bar-pct'>
                  <span className='result__bar-pct-num'>{absPct}%</span>
                </span>
              </div>
              <div className='result__bar-track'>
                <div className='result__bar-fill' style={{ width: `${absPct}%` }} />
              </div>
            </li>
          </ul>
        </div>

        <div className='hist-detail__col'>
          <div className='hist-detail__head'>Lifecycle</div>
          <MiniTimeline stages={STAGES} />
          <dl className='hist-detail__dl'>
            <dt>Ballots tallied</dt>
            <dd className='mono'>{entry.ballotCount.toLocaleString()}</dd>
            <dt>Open for</dt>
            <dd className='mono'>{entry.duration}</dd>
            <dt>Closed</dt>
            <dd className='mono'>{entry.closed}</dd>
            <dt>Privacy</dt>
            <dd>No individual ballot was ever decrypted.</dd>
          </dl>
        </div>
      </div>

      <div className='hist-detail__foot'>
        <a
          className='link-inline'
          href='#'
          onClick={(e) => {
            e.preventDefault()
            onNavigate?.('inspector')
          }}
        >
          Open full inspector view
          <span aria-hidden='true'>→</span>
        </a>
      </div>
    </div>
  )
}

function HistoryRow({
  entry,
  expanded,
  onToggle,
  onNavigate,
}: {
  entry: Entry
  expanded: boolean
  onToggle: () => void
  onNavigate?: (view: string) => void
}) {
  const [winner, pct] = entry.result.split(' · ')
  const declined = winner.toLowerCase().includes('declined')
  return (
    <li className={`hist-row ${expanded ? 'hist-row--open' : ''}`}>
      <button type='button' className='hist-row__btn' onClick={onToggle} aria-expanded={expanded}>
        <div className='hist-row__main'>
          <div className='hist-row__meta'>
            <span className='hist-row__id mono'>{entry.id}</span>
            <span className='hist-row__sep'>·</span>
            <span className='hist-row__date'>{entry.closed}</span>
            <span className='hist-row__sep'>·</span>
            <span className='hist-row__dur'>{entry.duration}</span>
          </div>
          <div className='hist-row__q'>{entry.question}</div>
        </div>
        <div className='hist-row__result'>
          <div className={`hist-row__verdict ${declined ? 'hist-row__verdict--declined' : ''}`}>
            <span className='hist-row__verdict-text'>{winner}</span>
            <span className='hist-row__verdict-pct mono'>{pct}</span>
          </div>
          <div className='hist-row__ballots mono'>{entry.ballotCount.toLocaleString()} ballots</div>
        </div>
        <span className={`hist-row__chev ${expanded ? 'hist-row__chev--open' : ''}`} aria-hidden='true'>
          <svg viewBox='0 0 12 12' width='12' height='12'>
            <path d='M4 2 L8 6 L4 10' fill='none' stroke='currentColor' strokeWidth='1.4' strokeLinecap='round' strokeLinejoin='round' />
          </svg>
        </span>
      </button>
      {expanded && <HistoryDetail entry={entry} onNavigate={onNavigate} />}
    </li>
  )
}

export default function History({ entries, onNavigate }: { entries: Entry[]; onNavigate?: (view: string) => void }) {
  const [filterId, setFilterId] = useState('all')
  const [expandedId, setExpandedId] = useState<string | null>(null)
  const filter = HIST_FILTERS.find((f) => f.id === filterId) || HIST_FILTERS[0]
  const filtered = entries.filter(filter.test as any)

  return (
    <section className='history' aria-label='Poll history'>
      <header className='history__head'>
        <div>
          <div className='section__eyebrow'>Archive</div>
          <h2 className='section__title'>Past polls</h2>
        </div>
        <div className='history__filters'>
          {HIST_FILTERS.map((f) => (
            <button key={f.id} type='button' className={`chip ${filterId === f.id ? 'chip--on' : ''}`} onClick={() => setFilterId(f.id)}>
              {f.label}
            </button>
          ))}
        </div>
      </header>
      <ul className='hist-list'>
        {filtered.map((e) => (
          <HistoryRow
            key={e.id}
            entry={e}
            expanded={expandedId === e.id}
            onToggle={() => setExpandedId((cur) => (cur === e.id ? null : e.id))}
            onNavigate={onNavigate}
          />
        ))}
        {filtered.length === 0 && <li className='hist-empty'>No polls match this filter.</li>}
      </ul>
      <div className='history__more'>
        <button className='link-btn'>
          Load earlier polls
          <span aria-hidden='true'>↓</span>
        </button>
      </div>
    </section>
  )
}
