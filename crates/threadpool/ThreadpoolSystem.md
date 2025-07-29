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
    style CRS fill:#C8E6C9
    style CRF fill:#C8E6C9
    style CR fill:#C8E6C9
    style EB fill:#FFCDD2
    style C fill:#FFCDD2

    click CRS "http://github.com/gnosisguild/enclave/tree/main/crates/events/docs/ComputeRequestSuccess.md"
    click CR "http://github.com/gnosisguild/enclave/tree/main/crates/events/docs/ComputeRequest.md"
    click CRF "http://github.com/gnosisguild/enclave/tree/main/crates/events/docs/ComputeRequestFailed.md"
    click EB "http://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click C "http://github.com/gnosisguild/enclave/tree/main/crates/threadpool/docs/ComputeProcessor.md"
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
