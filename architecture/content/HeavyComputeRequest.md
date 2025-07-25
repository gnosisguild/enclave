---
type: event
description: Requests a heavy computation that will require parallelism
---
 - `type:HeavyComputeJobType`
 - `payload:Arc<Zeroized<T>>` all payloads must be `Arc<Zeroized>>` so they are dropped once no longer needed.