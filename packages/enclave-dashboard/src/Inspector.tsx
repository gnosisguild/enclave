// E3 Inspector — deep technical record of a single E3.

import React, { useEffect, useRef, useState } from 'react'
import { STAGES, E3_DETAILS, E3_LIST } from './data'
import { CONTRACTS } from './lib/chain'
import { explorerAddress } from './lib/links'

export type InspectorE3List = Array<{ id: string; label: string }>

function Mono({ children, className = '' }: { children: React.ReactNode; className?: string }) {
  return <span className={`mono ${className}`}>{children}</span>
}

function CopyableHash({ value, full }: { value: string; full?: string }) {
  const [copied, setCopied] = useState(false)
  const onClick = () => {
    if (navigator.clipboard) navigator.clipboard.writeText(full || value)
    setCopied(true)
    setTimeout(() => setCopied(false), 1100)
  }
  return (
    <button type='button' className={`hash ${copied ? 'hash--copied' : ''}`} onClick={onClick} title={full || value}>
      <span className='mono'>{value}</span>
      <span className='hash__icon' aria-hidden='true'>
        {copied ? (
          <svg viewBox='0 0 12 12' width='11' height='11'>
            <path d='M2 6.5 L5 9 L10 3' fill='none' stroke='currentColor' strokeWidth='1.6' strokeLinecap='round' strokeLinejoin='round' />
          </svg>
        ) : (
          <svg viewBox='0 0 12 12' width='11' height='11'>
            <rect x='3.5' y='3.5' width='6.5' height='6.5' rx='1' fill='none' stroke='currentColor' strokeWidth='1.1' />
            <path d='M2 7.5 V2 H7.5' fill='none' stroke='currentColor' strokeWidth='1.1' />
          </svg>
        )}
      </span>
    </button>
  )
}

function InspStatusBadge({ stageIdx }: { stageIdx: number }) {
  const s = STAGES[stageIdx]
  const variant = stageIdx >= 6 ? 'published' : stageIdx === 3 ? 'open' : 'working'
  return (
    <span className={`stage-badge stage-badge--${variant}`}>
      <span className='stage-badge__dot' />
      <span>{s.label}</span>
    </span>
  )
}

function DefList({ items }: { items: Array<[React.ReactNode, React.ReactNode]> }) {
  return (
    <dl className='dl'>
      {items.map(([k, v], i) => (
        <React.Fragment key={i}>
          <dt>{k}</dt>
          <dd>{v}</dd>
        </React.Fragment>
      ))}
    </dl>
  )
}

function SectionCard({
  eyebrow,
  title,
  status,
  children,
  dense,
}: {
  eyebrow: string
  title: string
  status?: { kind: string; label: string }
  children: React.ReactNode
  dense?: boolean
}) {
  return (
    <section className={`isection ${dense ? 'isection--dense' : ''}`}>
      <header className='isection__head'>
        <div>
          <div className='isection__eyebrow'>{eyebrow}</div>
          <h3 className='isection__title'>{title}</h3>
        </div>
        {status && <span className={`isection__status isection__status--${status.kind}`}>{status.label}</span>}
      </header>
      <div className='isection__body'>{children}</div>
    </section>
  )
}

function InspectorStageStrip({ stages, currentStageIdx }: { stages: typeof STAGES; currentStageIdx: number }) {
  const wrapRef = useRef<HTMLDivElement | null>(null)
  const [overflow, setOverflow] = useState(false)
  useEffect(() => {
    const el = wrapRef.current
    if (!el) return
    const check = () => {
      const inner = el.querySelector('.istrip') as HTMLElement | null
      if (!inner) return
      setOverflow(inner.scrollWidth - inner.clientWidth > 4 && inner.scrollLeft + inner.clientWidth < inner.scrollWidth - 4)
    }
    check()
    const ro = new ResizeObserver(check)
    ro.observe(el)
    const inner = el.querySelector('.istrip')
    inner?.addEventListener('scroll', check)
    return () => {
      ro.disconnect()
      inner?.removeEventListener('scroll', check)
    }
  }, [])
  return (
    <div className='insp-head__strip' ref={wrapRef} data-overflow={overflow ? '1' : '0'}>
      <div className='istrip' role='list'>
        {stages.map((s, i) => {
          const state = i < currentStageIdx ? 'done' : i === currentStageIdx ? 'active' : 'todo'
          return (
            <React.Fragment key={s.id}>
              <div role='listitem' className={`istrip__node istrip__node--${state}`}>
                <span className='istrip__dot' />
                <span className='istrip__label'>{s.label}</span>
              </div>
              {i < stages.length - 1 && <span className={`istrip__rule ${i < currentStageIdx ? 'istrip__rule--done' : ''}`} />}
            </React.Fragment>
          )
        })}
      </div>
    </div>
  )
}

function EventLog({ events }: { events: any[] }) {
  const [filter, setFilter] = useState('all')
  const stages = ['all', ...Array.from(new Set(events.map((e) => e.stage)))]
  const filtered = filter === 'all' ? events : events.filter((e) => e.stage === filter)
  return (
    <div className='evlog-wrap'>
      <div className='evlog__filters'>
        {stages.map((s) => (
          <button key={s} type='button' className={`chip chip--sm ${filter === s ? 'chip--on' : ''}`} onClick={() => setFilter(s)}>
            {s === 'all' ? 'All events' : s}
          </button>
        ))}
        <span className='evlog__count mono'>
          {filtered.length} / {events.length}
        </span>
      </div>
      <table className='evlog'>
        <thead>
          <tr>
            <th>Time</th>
            <th>Block</th>
            <th>Event</th>
            <th>Stage</th>
            <th>Tx</th>
            <th className='evlog__num'>Gas</th>
          </tr>
        </thead>
        <tbody>
          {filtered.map((ev, i) => (
            <tr key={i}>
              <td>
                <Mono>{ev.t}</Mono>
              </td>
              <td>
                <Mono>{typeof ev.block === 'number' ? `#${ev.block.toLocaleString()}` : ev.block}</Mono>
              </td>
              <td>
                <span className='evlog__name'>{ev.name}</span>
              </td>
              <td>
                <span className='evlog__stage'>{ev.stage}</span>
              </td>
              <td>{ev.tx === '—' ? <Mono>—</Mono> : <CopyableHash value={ev.tx} />}</td>
              <td className='evlog__num mono'>{ev.gas}</td>
            </tr>
          ))}
          {filtered.length === 0 && (
            <tr>
              <td colSpan={6} className='evlog__empty'>
                No events match this filter.
              </td>
            </tr>
          )}
        </tbody>
      </table>
      <div className='evlog__foot'>
        <span>Events stream live from the Enclave contract on Sepolia.</span>
        <a className='link-inline' href={explorerAddress(CONTRACTS.Enclave) + '#events'} target='_blank' rel='noreferrer'>
          Open in block explorer →
        </a>
      </div>
    </div>
  )
}

export default function Inspector({
  e3List: e3ListProp,
  e3Override,
  selectedId: selectedIdProp,
  onSelect,
  loading,
  error,
}: {
  e3List?: InspectorE3List
  e3Override?: any
  selectedId?: string
  onSelect?: (id: string) => void
  loading?: boolean
  error?: Error | null
} = {}) {
  const list = e3ListProp && e3ListProp.length > 0 ? e3ListProp : E3_LIST
  const fallbackId = list[0]?.id ?? 'E3-0481'
  const [localId, setLocalId] = useState(fallbackId)
  const selectedId = selectedIdProp ?? localId
  const setSelectedId = onSelect ?? setLocalId
  const e3 = e3Override ?? E3_DETAILS[selectedId] ?? E3_DETAILS['E3-0481']

  return (
    <div className='inspector'>
      {(loading || error) && (
        <div
          style={{
            padding: '10px 14px',
            margin: '0 0 12px',
            borderRadius: 8,
            fontSize: 12,
            background: error ? '#fff1f0' : '#f4f6f8',
            color: error ? '#8a1f1f' : '#3a3f4a',
          }}
        >
          {error ? `Failed to load on-chain data: ${error.message}. Showing cached preview.` : 'Loading on-chain data from Sepolia…'}
        </div>
      )}
      <section className='insp-head'>
        <div className='insp-head__row'>
          <div>
            <div className='insp-head__breadcrumb'>
              <span>Network</span>
              <span className='insp-head__crumb-sep'>/</span>
              <span>E3 inspector</span>
              <span className='insp-head__crumb-sep'>/</span>
              <Mono>{e3.id}</Mono>
            </div>
            <h1 className='insp-head__title'>{e3.summary}</h1>
            <div className='insp-head__meta'>
              <span>Program</span>
              <Mono>{e3.program}</Mono>
              <span className='insp-head__meta-sep'>·</span>
              <span>Requested by</span>
              <Mono>
                {e3.requestedByLabel} · {e3.requestedBy}
              </Mono>
            </div>
          </div>
          <div className='insp-head__selector'>
            <label htmlFor='e3-select' className='insp-head__selector-label'>
              Inspect E3
            </label>
            <select id='e3-select' className='insp-select' value={selectedId} onChange={(e) => setSelectedId(e.target.value)}>
              {list.map((e) => (
                <option key={e.id} value={e.id}>
                  {e.id} · {e.label}
                </option>
              ))}
            </select>
          </div>
        </div>

        <InspectorStageStrip stages={STAGES} currentStageIdx={e3.currentStage} />

        <div className='insp-stats'>
          <div className='insp-stat'>
            <div className='insp-stat__label'>Status</div>
            <div className='insp-stat__value'>
              <InspStatusBadge stageIdx={e3.currentStage} />
            </div>
          </div>
          <div className='insp-stat'>
            <div className='insp-stat__label'>Committee</div>
            <div className='insp-stat__value mono'>
              {e3.committee.threshold} <span className='insp-stat__of'>of</span> {e3.committee.size}
            </div>
            <div className='insp-stat__sub'>threshold · total nodes</div>
          </div>
          <div className='insp-stat'>
            <div className='insp-stat__label'>Ballots</div>
            <div className='insp-stat__value mono'>{e3.input.ballotsReceived.toLocaleString()}</div>
            <div className='insp-stat__sub'>encrypted · in-flight</div>
          </div>
          <div className='insp-stat'>
            <div className='insp-stat__label'>Compute fee</div>
            <div className='insp-stat__value mono'>{e3.fees.computeFee}</div>
            <div className='insp-stat__sub'>of {e3.fees.requesterDeposit} deposit</div>
          </div>
        </div>
      </section>

      <SectionCard eyebrow='01 · Request & Committee' title='How this E3 came into being'>
        <div className='isection__grid'>
          <DefList
            items={[
              ['Requested at', <Mono>{e3.requestedAt}</Mono>],
              ['Request tx', <CopyableHash value={`${e3.requestedTx.slice(0, 10)}…${e3.requestedTx.slice(-6)}`} full={e3.requestedTx} />],
              ['Block', <Mono>#{e3.requestedBlock.toLocaleString()}</Mono>],
              ['Requested by', <Mono>{e3.requestedBy}</Mono>],
              ['Program', <Mono>{e3.program}</Mono>],
              ['Program address', <Mono>{e3.programAddr}</Mono>],
            ]}
          />
          <DefList
            items={[
              ['Committee size', <Mono>{e3.committee.size} nodes</Mono>],
              [
                'Decryption threshold',
                <Mono>
                  {e3.committee.threshold} of {e3.committee.size}
                </Mono>,
              ],
              ['Selection seed', <Mono>{e3.committee.selectionSeed}</Mono>],
              ['Selection tx', <CopyableHash value={e3.committee.selectionTx} />],
              ['Drawn at', <Mono>{e3.committee.drawnAt}</Mono>],
              ['Identities', <span className='dl__muted'>{e3.committee.note}</span>],
            ]}
          />
        </div>
      </SectionCard>

      <SectionCard eyebrow='02 · Keygen' title='Distributed key generation' status={{ kind: 'done', label: 'Complete' }}>
        <p className='isection__lede'>
          The committee jointly produced an encryption key in three rounds. The matching <em>decryption</em> key is held in shares — never
          assembled, never written down.
        </p>
        <div className='kg-protocol'>
          <span className='kg-protocol__label'>Protocol</span>
          <Mono>{e3.keygen.protocol}</Mono>
        </div>
        <ol className='kg-rounds'>
          {e3.keygen.rounds.map((r: any, i: number) => (
            <li key={i} className={`kg-round kg-round--${r.status}`}>
              <div className='kg-round__num mono'>{String(i + 1).padStart(2, '0')}</div>
              <div className='kg-round__body'>
                <div className='kg-round__top'>
                  <div className='kg-round__name'>{r.name}</div>
                  <div className='kg-round__chips'>
                    <span className='kg-chip'>
                      <span className='kg-chip__k'>Participants</span>
                      <span className='mono'>{r.participants}</span>
                    </span>
                    <span className='kg-chip'>
                      <span className='kg-chip__k'>Started</span>
                      <span className='mono'>{r.startedAt}</span>
                    </span>
                    <span className='kg-chip'>
                      <span className='kg-chip__k'>Duration</span>
                      <span className='mono'>{r.duration}</span>
                    </span>
                    <span className='kg-chip kg-chip--tx'>
                      <CopyableHash value={r.tx} />
                    </span>
                  </div>
                </div>
                <div className='kg-round__note'>{r.note}</div>
              </div>
            </li>
          ))}
        </ol>
        <div className='kg-pk'>
          <span className='kg-pk__label'>Joint public key</span>
          <Mono>{e3.keygen.publicKey}</Mono>
        </div>
      </SectionCard>

      <SectionCard eyebrow='03 · Input Window' title='Encrypted ballots received' status={{ kind: 'live', label: 'Active' }}>
        <div className='isection__grid'>
          <DefList
            items={[
              ['Opened', <Mono>{e3.input.openedAt}</Mono>],
              ['Closes', <Mono>{e3.input.closesAt}</Mono>],
              ['Ballots received', <Mono>{e3.input.ballotsReceived.toLocaleString()}</Mono>],
            ]}
          />
          <DefList
            items={[
              ['First ballot', <Mono>{e3.input.firstBallotAt}</Mono>],
              ['Last ballot', <Mono>{e3.input.lastBallotAt}</Mono>],
              ['Avg ballot size', <Mono>{e3.input.avgBallotSize}</Mono>],
              ['Ballot circuit', <Mono>{e3.input.ballotCircuit}</Mono>],
            ]}
          />
        </div>
      </SectionCard>

      <SectionCard eyebrow='04 · Compute' title='Homomorphic tally' status={{ kind: 'pending', label: 'Pending' }}>
        <p className='isection__lede'>{e3.compute.note}</p>
        <DefList
          items={[
            ['Tally circuit', <Mono>{e3.compute.circuit}</Mono>],
            ['Estimated duration', <Mono>{e3.compute.estDuration}</Mono>],
            ['Estimated gas', <Mono>{e3.compute.estGas}</Mono>],
          ]}
        />
      </SectionCard>

      <SectionCard eyebrow='05 · Decryption' title='Threshold decryption' status={{ kind: 'pending', label: 'Pending' }}>
        <p className='isection__lede'>{e3.decryption.note}</p>
        <div className='dec-progress'>
          <div className='dec-progress__head'>
            <span>Partial decryptions received</span>
            <Mono>
              {e3.decryption.sharesReceived} / {e3.decryption.sharesRequired}
            </Mono>
          </div>
          <div className='dec-progress__bar'>
            <div
              className='dec-progress__fill'
              style={{
                width: `${(e3.decryption.sharesReceived / e3.decryption.sharesRequired) * 100}%`,
              }}
            />
          </div>
          <div className='dec-progress__sub'>
            {e3.decryption.sharesRequired} of {e3.committee.size} committee members must each publish a partial share.
          </div>
        </div>
      </SectionCard>

      <SectionCard eyebrow='06 · Publication' title='Result on-chain' status={{ kind: 'pending', label: 'Pending' }}>
        <p className='isection__lede'>{e3.publication.note}</p>
      </SectionCard>

      <SectionCard eyebrow='07 · Fees & settlement' title='Where the deposit goes'>
        <div className='fees'>
          <div className='fees__split' aria-hidden='true'>
            <div
              className='fees__split-seg fees__split-seg--committee'
              style={{ flex: 0.0312 }}
              title={`Committee reward · ${e3.fees.committeeReward}`}
            />
            <div
              className='fees__split-seg fees__split-seg--network'
              style={{ flex: 0.0084 }}
              title={`Network fee · ${e3.fees.networkFee}`}
            />
            <div
              className='fees__split-seg fees__split-seg--refund'
              style={{ flex: 0.454 }}
              title={`Refundable · ${e3.fees.refundAvailable}`}
            />
          </div>
          <ul className='fees__legend'>
            <li>
              <span className='fees__swatch fees__swatch--committee' />
              <div className='fees__legend-body'>
                <div className='fees__legend-k'>Committee reward</div>
                <div className='mono fees__legend-v'>{e3.fees.committeeReward}</div>
                <div className='fees__legend-pct'>6.2% of deposit</div>
              </div>
            </li>
            <li>
              <span className='fees__swatch fees__swatch--network' />
              <div className='fees__legend-body'>
                <div className='fees__legend-k'>Network fee</div>
                <div className='mono fees__legend-v'>{e3.fees.networkFee}</div>
                <div className='fees__legend-pct'>1.7% of deposit</div>
              </div>
            </li>
            <li>
              <span className='fees__swatch fees__swatch--refund' />
              <div className='fees__legend-body'>
                <div className='fees__legend-k'>Refundable to requester</div>
                <div className='mono fees__legend-v'>{e3.fees.refundAvailable}</div>
                <div className='fees__legend-pct'>90.8% of deposit</div>
              </div>
            </li>
          </ul>
          <DefList
            items={[
              ['Requester deposit', <Mono>{e3.fees.requesterDeposit}</Mono>],
              ['Compute fee', <Mono>{e3.fees.computeFee}</Mono>],
              ['Settlement', <span className='dl__muted'>{e3.fees.currency}</span>],
            ]}
          />
        </div>
      </SectionCard>

      <SectionCard eyebrow='08 · Event log' title='On-chain events, oldest first' dense>
        <EventLog events={e3.events} />
      </SectionCard>
    </div>
  )
}
