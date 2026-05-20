// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
// Tweak state hook, split out from tweaks-panel.tsx so that file only exports
// React components (keeps Fast Refresh happy).

import React from 'react'

export function useTweaks<T extends Record<string, any>>(defaults: T) {
  const [values, setValues] = React.useState<T>(defaults)
  const setTweak = React.useCallback((keyOrEdits: any, val?: any) => {
    const edits = typeof keyOrEdits === 'object' && keyOrEdits !== null ? keyOrEdits : { [keyOrEdits]: val }
    setValues((prev) => ({ ...prev, ...edits }))
    try {
      window.parent.postMessage({ type: '__edit_mode_set_keys', edits }, '*')
    } catch {
      // postMessage is unavailable when not embedded in a host frame
      void 0
    }
    window.dispatchEvent(new CustomEvent('tweakchange', { detail: edits }))
  }, [])
  return [values, setTweak] as const
}
