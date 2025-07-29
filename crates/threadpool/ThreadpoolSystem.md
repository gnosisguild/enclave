---
type: system
description: Manages multi-threaded compute by ensuring compute is run on a rayon threadpool
tags:
  - todo
  - compute
---

## `=this.file.name`

`=this.description`

```mermaid
flowchart TB
    subgraph subGraph0["Threadpool System"]
        CRS["ComputeRequestSuccess"]
        CRF["ComputeRequestFailed"]
        CR["ComputeRequest"]
        EB["EventBus"]
        C["ThreadpoolComputeProcessor"]
    end
    C --> CRS
    C --> CRF
    EB --> CR
    CR --> C
    CRS@{ shape: event}
    CRF@{ shape: event}
    CR@{ shape: event}
    CRS:::internal-link
    CR:::internal-link
	CRF:::internal-link
    EB:::internal-link
    C:::internal-link

    click CRS "https://github.com/gnosisguild/enclave/tree/main/crates/events/docs/ComputeRequestSuccess.md"
    click CR "https://github.com/gnosisguild/enclave/tree/main/crates/events/docs/ComputeRequest.md"
    click CRF "https://github.com/gnosisguild/enclave/tree/main/crates/events/docs/ComputeRequestFailed.md"
    click EB "https://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click C "https://github.com/gnosisguild/enclave/tree/main/crates/threadpool/docs/ThreadpoolComputeProcessor.md"
```
<details>
<summary>Links</summary>

[[ComputeRequestFailed]]
[[ComputeRequestSuccess]]
[[ComputeRequest]]
[[EventBus]]
[[ThreadpoolComputeProcessor]]
</details>

```dataview
TABLE type, description as Description
FROM #compute
```
