// tweaks-panel.tsx
// Reusable Tweaks shell + form-control helpers ported from the Claude Design
// starter runtime. The host-protocol postMessage bridge (used inside Claude
// Design) is preserved but is a harmless no-op when this app runs standalone.

import React from 'react'

const __TWEAKS_STYLE = `
  .twk-panel{position:fixed;right:16px;bottom:16px;z-index:2147483646;width:280px;
    max-height:calc(100vh - 32px);display:flex;flex-direction:column;
    transform:scale(var(--dc-inv-zoom,1));transform-origin:bottom right;
    background:rgba(250,249,247,.78);color:#29261b;
    -webkit-backdrop-filter:blur(24px) saturate(160%);backdrop-filter:blur(24px) saturate(160%);
    border:.5px solid rgba(255,255,255,.6);border-radius:14px;
    box-shadow:0 1px 0 rgba(255,255,255,.5) inset,0 12px 40px rgba(0,0,0,.18);
    font:11.5px/1.4 ui-sans-serif,system-ui,-apple-system,sans-serif;overflow:hidden}
  .twk-hd{display:flex;align-items:center;justify-content:space-between;
    padding:10px 8px 10px 14px;cursor:move;user-select:none}
  .twk-hd b{font-size:12px;font-weight:600;letter-spacing:.01em}
  .twk-x{appearance:none;border:0;background:transparent;color:rgba(41,38,27,.55);
    width:22px;height:22px;border-radius:6px;cursor:default;font-size:13px;line-height:1}
  .twk-x:hover{background:rgba(0,0,0,.06);color:#29261b}
  .twk-body{padding:2px 14px 14px;display:flex;flex-direction:column;gap:10px;
    overflow-y:auto;overflow-x:hidden;min-height:0;
    scrollbar-width:thin;scrollbar-color:rgba(0,0,0,.15) transparent}
  .twk-body::-webkit-scrollbar{width:8px}
  .twk-body::-webkit-scrollbar-track{background:transparent;margin:2px}
  .twk-body::-webkit-scrollbar-thumb{background:rgba(0,0,0,.15);border-radius:4px;
    border:2px solid transparent;background-clip:content-box}
  .twk-body::-webkit-scrollbar-thumb:hover{background:rgba(0,0,0,.25);
    border:2px solid transparent;background-clip:content-box}
  .twk-row{display:flex;flex-direction:column;gap:5px}
  .twk-row-h{flex-direction:row;align-items:center;justify-content:space-between;gap:10px}
  .twk-lbl{display:flex;justify-content:space-between;align-items:baseline;
    color:rgba(41,38,27,.72)}
  .twk-lbl>span:first-child{font-weight:500}
  .twk-val{color:rgba(41,38,27,.5);font-variant-numeric:tabular-nums}

  .twk-sect{font-size:10px;font-weight:600;letter-spacing:.06em;text-transform:uppercase;
    color:rgba(41,38,27,.45);padding:10px 0 0}
  .twk-sect:first-child{padding-top:0}

  .twk-field{appearance:none;box-sizing:border-box;width:100%;min-width:0;height:26px;padding:0 8px;
    border:.5px solid rgba(0,0,0,.1);border-radius:7px;
    background:rgba(255,255,255,.6);color:inherit;font:inherit;outline:none}
  .twk-field:focus{border-color:rgba(0,0,0,.25);background:rgba(255,255,255,.85)}
  select.twk-field{padding-right:22px;
    background-image:url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6'><path fill='rgba(0,0,0,.5)' d='M0 0h10L5 6z'/></svg>");
    background-repeat:no-repeat;background-position:right 8px center}

  .twk-slider{appearance:none;-webkit-appearance:none;width:100%;height:4px;margin:6px 0;
    border-radius:999px;background:rgba(0,0,0,.12);outline:none}
  .twk-slider::-webkit-slider-thumb{-webkit-appearance:none;appearance:none;
    width:14px;height:14px;border-radius:50%;background:#fff;
    border:.5px solid rgba(0,0,0,.12);box-shadow:0 1px 3px rgba(0,0,0,.2);cursor:default}
  .twk-slider::-moz-range-thumb{width:14px;height:14px;border-radius:50%;
    background:#fff;border:.5px solid rgba(0,0,0,.12);box-shadow:0 1px 3px rgba(0,0,0,.2);cursor:default}

  .twk-seg{position:relative;display:flex;padding:2px;border-radius:8px;
    background:rgba(0,0,0,.06);user-select:none}
  .twk-seg-thumb{position:absolute;top:2px;bottom:2px;border-radius:6px;
    background:rgba(255,255,255,.9);box-shadow:0 1px 2px rgba(0,0,0,.12);
    transition:left .15s cubic-bezier(.3,.7,.4,1),width .15s}
  .twk-seg.dragging .twk-seg-thumb{transition:none}
  .twk-seg button{appearance:none;position:relative;z-index:1;flex:1;border:0;
    background:transparent;color:inherit;font:inherit;font-weight:500;min-height:22px;
    border-radius:6px;cursor:default;padding:4px 6px;line-height:1.2;
    overflow-wrap:anywhere}

  .twk-toggle{position:relative;width:32px;height:18px;border:0;border-radius:999px;
    background:rgba(0,0,0,.15);transition:background .15s;cursor:default;padding:0}
  .twk-toggle[data-on="1"]{background:#34c759}
  .twk-toggle i{position:absolute;top:2px;left:2px;width:14px;height:14px;border-radius:50%;
    background:#fff;box-shadow:0 1px 2px rgba(0,0,0,.25);transition:transform .15s}
  .twk-toggle[data-on="1"] i{transform:translateX(14px)}
`

export function TweaksPanel({ title = 'Tweaks', children }: { title?: string; children?: React.ReactNode }) {
  const [open, setOpen] = React.useState(true)
  const dragRef = React.useRef<HTMLDivElement | null>(null)
  const offsetRef = React.useRef({ x: 16, y: 16 })
  const PAD = 16

  const clampToViewport = React.useCallback(() => {
    const panel = dragRef.current
    if (!panel) return
    const w = panel.offsetWidth
    const h = panel.offsetHeight
    const maxRight = Math.max(PAD, window.innerWidth - w - PAD)
    const maxBottom = Math.max(PAD, window.innerHeight - h - PAD)
    offsetRef.current = {
      x: Math.min(maxRight, Math.max(PAD, offsetRef.current.x)),
      y: Math.min(maxBottom, Math.max(PAD, offsetRef.current.y)),
    }
    panel.style.right = offsetRef.current.x + 'px'
    panel.style.bottom = offsetRef.current.y + 'px'
  }, [])

  React.useEffect(() => {
    if (!open) return
    clampToViewport()
    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', clampToViewport)
      return () => window.removeEventListener('resize', clampToViewport)
    }
    const ro = new ResizeObserver(clampToViewport)
    ro.observe(document.documentElement)
    return () => ro.disconnect()
  }, [open, clampToViewport])

  React.useEffect(() => {
    const onMsg = (e: MessageEvent) => {
      const t = (e as any)?.data?.type
      if (t === '__activate_edit_mode') setOpen(true)
      else if (t === '__deactivate_edit_mode') setOpen(false)
    }
    window.addEventListener('message', onMsg)
    try {
      window.parent.postMessage({ type: '__edit_mode_available' }, '*')
    } catch {
      void 0
    }
    return () => window.removeEventListener('message', onMsg)
  }, [])

  const dismiss = () => {
    setOpen(false)
    try {
      window.parent.postMessage({ type: '__edit_mode_dismissed' }, '*')
    } catch {
      void 0
    }
  }

  const onDragStart = (e: React.MouseEvent) => {
    const panel = dragRef.current
    if (!panel) return
    const r = panel.getBoundingClientRect()
    const sx = e.clientX
    const sy = e.clientY
    const startRight = window.innerWidth - r.right
    const startBottom = window.innerHeight - r.bottom
    const move = (ev: MouseEvent) => {
      offsetRef.current = {
        x: startRight - (ev.clientX - sx),
        y: startBottom - (ev.clientY - sy),
      }
      clampToViewport()
    }
    const up = () => {
      window.removeEventListener('mousemove', move)
      window.removeEventListener('mouseup', up)
    }
    window.addEventListener('mousemove', move)
    window.addEventListener('mouseup', up)
  }

  if (!open) return null
  return (
    <>
      <style>{__TWEAKS_STYLE}</style>
      <div ref={dragRef} className='twk-panel' style={{ right: PAD, bottom: PAD }}>
        <div className='twk-hd' onMouseDown={onDragStart}>
          <b>{title}</b>
          <button className='twk-x' aria-label='Close tweaks' onMouseDown={(e) => e.stopPropagation()} onClick={dismiss}>
            ✕
          </button>
        </div>
        <div className='twk-body'>{children}</div>
      </div>
    </>
  )
}

export function TweakSection({ label, children }: { label: string; children?: React.ReactNode }) {
  return (
    <>
      <div className='twk-sect'>{label}</div>
      {children}
    </>
  )
}

function TweakRow({
  label,
  value,
  children,
  inline = false,
}: {
  label: string
  value?: any
  children?: React.ReactNode
  inline?: boolean
}) {
  return (
    <div className={inline ? 'twk-row twk-row-h' : 'twk-row'}>
      <div className='twk-lbl'>
        <span>{label}</span>
        {value != null && <span className='twk-val'>{value}</span>}
      </div>
      {children}
    </div>
  )
}

export function TweakToggle({ label, value, onChange }: { label: string; value: boolean; onChange: (v: boolean) => void }) {
  return (
    <div className='twk-row twk-row-h'>
      <div className='twk-lbl'>
        <span>{label}</span>
      </div>
      <button
        type='button'
        className='twk-toggle'
        data-on={value ? '1' : '0'}
        role='switch'
        aria-checked={!!value}
        onClick={() => onChange(!value)}
      >
        <i />
      </button>
    </div>
  )
}

export function TweakSelect({
  label,
  value,
  options,
  onChange,
}: {
  label: string
  value: string
  options: Array<{ value: string; label: string } | string>
  onChange: (v: string) => void
}) {
  return (
    <TweakRow label={label}>
      <select className='twk-field' value={value} onChange={(e) => onChange(e.target.value)}>
        {options.map((o) => {
          const v = typeof o === 'object' ? o.value : o
          const l = typeof o === 'object' ? o.label : o
          return (
            <option key={v} value={v}>
              {l}
            </option>
          )
        })}
      </select>
    </TweakRow>
  )
}

export function TweakRadio({
  label,
  value,
  options,
  onChange,
}: {
  label: string
  value: string
  options: Array<{ value: string; label: string } | string>
  onChange: (v: string) => void
}) {
  const trackRef = React.useRef<HTMLDivElement | null>(null)
  const [dragging, setDragging] = React.useState(false)
  const valueRef = React.useRef(value)
  React.useEffect(() => {
    valueRef.current = value
  }, [value])

  const labelLen = (o: any) => String(typeof o === 'object' ? o.label : o).length
  const maxLen = options.reduce((m: number, o: any) => Math.max(m, labelLen(o)), 0)
  const fitsAsSegments = maxLen <= (({ 2: 16, 3: 10 } as Record<number, number>)[options.length] ?? 0)

  if (!fitsAsSegments) {
    const resolve = (s: string) => {
      const m = options.find((o: any) => String(typeof o === 'object' ? o.value : o) === s)
      return m === undefined ? s : typeof m === 'object' ? (m as any).value : m
    }
    return <TweakSelect label={label} value={value} options={options} onChange={(s) => onChange(resolve(s))} />
  }
  const opts = options.map((o: any) => (typeof o === 'object' ? o : { value: o, label: o }))
  const idx = Math.max(
    0,
    opts.findIndex((o: any) => o.value === value),
  )
  const n = opts.length

  const segAt = (clientX: number) => {
    const r = trackRef.current!.getBoundingClientRect()
    const inner = r.width - 4
    const i = Math.floor(((clientX - r.left - 2) / inner) * n)
    return opts[Math.max(0, Math.min(n - 1, i))].value
  }

  const onPointerDown = (e: React.PointerEvent) => {
    setDragging(true)
    const v0 = segAt(e.clientX)
    if (v0 !== valueRef.current) onChange(v0)
    const move = (ev: PointerEvent) => {
      if (!trackRef.current) return
      const v = segAt(ev.clientX)
      if (v !== valueRef.current) onChange(v)
    }
    const up = () => {
      setDragging(false)
      window.removeEventListener('pointermove', move)
      window.removeEventListener('pointerup', up)
    }
    window.addEventListener('pointermove', move)
    window.addEventListener('pointerup', up)
  }

  return (
    <TweakRow label={label}>
      <div ref={trackRef} role='radiogroup' onPointerDown={onPointerDown} className={dragging ? 'twk-seg dragging' : 'twk-seg'}>
        <div
          className='twk-seg-thumb'
          style={{
            left: `calc(2px + ${idx} * (100% - 4px) / ${n})`,
            width: `calc((100% - 4px) / ${n})`,
          }}
        />
        {opts.map((o: any) => (
          <button key={o.value} type='button' role='radio' aria-checked={o.value === value}>
            {o.label}
          </button>
        ))}
      </div>
    </TweakRow>
  )
}
