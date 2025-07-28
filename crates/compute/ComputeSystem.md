---
type: system
description: Manages multithread threaded compute by ensuring compute is run on a rayon threadpool
tags:
  - todo
  - compute
---
## `=this.file.name`

`=this.description`


```mermaid
flowchart TB
    subgraph subGraph0["Compute System"]
        CRS["ComputeRequestSuccess"]
        CRF["ComputeRequestFailed"]
        CR["ComputeRequest"]
        EB["EventBus"]
        C["ComputeProcessor"]
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

    click CRS "https://github.com/gnosisguild/enclave/tree/main/crates/events/docs/ComputeRequestSuccess.md"
    click CR "https://github.com/gnosisguild/enclave/tree/main/crates/events/docs/ComputeRequest.md"
    click CRF "https://github.com/gnosisguild/enclave/tree/main/architecture/content/.trash/ComputeRequestFailed.md"
    click EB "https://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click C "https://github.com/gnosisguild/enclave/tree/main/crates/compute/docs/ComputeProcessor.md"
```

```dataview
TABLE type, description as Description
FROM #compute
```
