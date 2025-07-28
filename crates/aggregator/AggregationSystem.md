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
	style EB fill:#FFCDD2
	style S fill:#FFCDD2
	style PT fill:#FFCDD2
	style PK fill:#FFCDD2
	style C fill:#FFCDD2

    click PT "https://github.com/gnosisguild/enclave/tree/main/crates/aggregator/docs/PlaintextAggregator.md"
    click PK "https://github.com/gnosisguild/enclave/tree/main/crates/aggregator/docs/PublickeyAggregator.md"
    click EB "https://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click C "https://github.com/gnosisguild/enclave/tree/main/crates/compute/docs/ComputeProcessor.md"
    click S "https://github.com/gnosisguild/enclave/tree/main/crates/sortition/docs/Sortition.md"
```


```dataview
TABLE type, description as Description
FROM #aggregator
```

