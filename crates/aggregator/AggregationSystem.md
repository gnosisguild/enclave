---
type: system
description: Components for aggregating shared data
tags:
  - aggregator
---

## `=this.file.name`

`=this.description`

```mermaid
flowchart TB
    subgraph subGraph0["Aggregation System"]
        PT["PlaintextAggregator"]
        PK["PublickeyAggregator"]
        EB["EventBus"]
        S["Sortition"]
        C["ComputeProcessor"]
    end
    EB --> PT
    EB --> PK
    PT --> S
    PK --> S
    PT --> C
    PK --> C
    PT:::internal-link
    PK:::internal-link
    EB:::internal-link
    C:::internal-link
	S:::internal-link

    click PT "http://github.com/gnosisguild/enclave/tree/main/crates/aggregator/docs/PlaintextAggregator.md"
    click PK "http://github.com/gnosisguild/enclave/tree/main/crates/aggregator/docs/PublickeyAggregator.md"
    click EB "http://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click C "http://github.com/gnosisguild/enclave/tree/main/crates/threadpool/docs/ThreadpoolComputeProcessor.md"
    click S "http://github.com/gnosisguild/enclave/tree/main/crates/sortition/docs/Sortition.md"
```
<details>
<summary>Links</summary>

[[ComputeProcessor]]
[[EventBus]]
[[PlaintextAggregator]]
[[PublickeyAggregator]]
[[Sortition]]
</details>

```dataview
TABLE type, description as Description
FROM #aggregator
```
