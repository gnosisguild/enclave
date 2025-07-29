---
type: system
description: Filter and forward events by e3_id and manage the e3 request content
tags:
  - e3request
---
```mermaid
flowchart TB
 subgraph subGraph0["E3Request System"]
        ER["E3Router"]
        EC["E3Context"]
        K["Keyshare"]
        PKA["PublickeyAggregator"]
        PTA["PlaintextAggregator"]
        EB["EventBus"]
        CS["CiphernodeSelector"]
  end
    EB --> CS
    CS --> ER
    ER --- EC
    ER -. filter(e3_id) <br> .-> K & PTA
    ER -. filter(e3_id) </br> .-> PKA

    EC@{ shape: cyl}
     EC:::internal-link
     ER:::internal-link
     PKA:::internal-link
     PTA:::internal-link
     K:::internal-link
     EB:::internal-link
     CS:::internal-link
    style ER fill:#FFCDD2
    style EC fill:#BBDEFB
    style K fill:#FFCDD2
    style PKA fill:#FFCDD2
    style PTA fill:#FFCDD2
    style EB fill:#FFCDD2
    style CS fill:#FFCDD2

    click EC "http://github.com/gnosisguild/enclave/tree/main/crates/request/docs/E3Context.md"
    click ER "http://github.com/gnosisguild/enclave/tree/main/crates/request/docs/E3Router.md"
    click PKA "http://github.com/gnosisguild/enclave/tree/main/crates/aggregator/docs/PublickeyAggregator.md"
    click PTA "http://github.com/gnosisguild/enclave/tree/main/crates/aggregator/docs/PlaintextAggregator.md"
    click K "http://github.com/gnosisguild/enclave/tree/main/crates/keyshare/docs/Keyshare.md"
    click EB "http://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click CS "http://github.com/gnosisguild/enclave/tree/main/crates/sortition/docs/CiphernodeSelector.md"
```
<details>
<summary><i>Links</i></summary>

[[CiphernodeSelector]]
[[E3Context]]
[[E3Router]]
[[EventBus]]
[[Keyshare]]
[[PlaintextAggregator]]
[[PublickeyAggregator]]
</details>
<details>
<summary><i>Links</i></summary>

[[CiphernodeSelector]]
[[E3Context]]
[[E3Router]]
[[EventBus]]
[[Keyshare]]
[[PlaintextAggregator]]
[[PublickeyAggregator]]
</details>

```dataview
TABLE type, description as Description
FROM #e3request
```
