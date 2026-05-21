// Keeps anvil block.timestamp moving during local integration (finalizeCommittee needs
// block.timestamp > committeeDeadline; eth_call-only retries do not mine blocks).

const RPC = process.env.ANVIL_RPC_URL ?? 'http://127.0.0.1:8545'

async function rpc(method, params = []) {
  const res = await fetch(RPC, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
  })
  if (!res.ok) {
    throw new Error(`RPC ${method} HTTP ${res.status}`)
  }
  const json = await res.json()
  if (json.error) {
    throw new Error(json.error.message ?? JSON.stringify(json.error))
  }
}

async function loop() {
  for (;;) {
    try {
      await rpc('evm_mine')
    } catch {
      // anvil not up yet
    }
    await new Promise((r) => setTimeout(r, 1000))
  }
}

loop()
