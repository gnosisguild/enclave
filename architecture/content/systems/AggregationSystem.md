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
```


```dataview
TABLE type, description as Description
FROM #aggregator
```

