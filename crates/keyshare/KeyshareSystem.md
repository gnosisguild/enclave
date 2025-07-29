---
type: system
description: Manages generating and manipulating keyshares
tags:
  - keyshare
---
## `=this.file.name`

`=this.description`

_note: in the following diagram dotted components are deprecated_

```mermaid
flowchart TB
	subgraph subGraph0["ThesholdKeyshare"]
	    KS["ThresholdKeyshare"]
	    S["Sensitive"]
	    EB["EventBus"]
	    C["ThreadpoolComputeProcessor"]
	    TRB["TrBFV"]
	    TFHE["TFHE"]
	    BFV["BFV"]
	    KSL["Keyshare"]
	end
    EB --> KS
    KS --> C
    KS --> S
    C --> TRB
    C --> S
	C --> TFHE
	EB --> KSL
    KSL --> BFV
		
	KS:::internal-link
	KSL:::internal-link
	EB:::internal-link
	S:::internal-link
	C:::internal-link
	TRB:::internal-link
	TFHE:::internal-link
	BFV:::internal-link
	
    style KSL stroke-width:1px,stroke-dasharray: 5
    style BFV stroke-width:1px,stroke-dasharray: 5

    click KS "https://github.com/gnosisguild/enclave/tree/main/crates/keyshare/docs/ThresholdKeyshare.md"
    click KSL "https://github.com/gnosisguild/enclave/tree/main/crates/keyshare/docs/Keyshare.md"
    click EB "https://github.com/gnosisguild/enclave/tree/main/crates/events/docs/EventBus.md"
    click S "https://github.com/gnosisguild/enclave/tree/main/crates/crypto/docs/Sensitive.md"
    click C "https://github.com/gnosisguild/enclave/tree/main/crates/threadpool/docs/ThreadpoolComputeProcessor.md"
    click TRB "https://github.com/gnosisguild/enclave/tree/main/crates/fhe/docs/TrBFV.md"
    click TFHE "https://github.com/gnosisguild/enclave/tree/main/crates/fhe/docs/TFHE.md"
    click BFV "https://github.com/gnosisguild/enclave/tree/main/crates/fhe/docs/Bfv.md"
```
<details>
<summary>Links</summary>

[[BFV]]
[[EventBus]]
[[Keyshare]]
[[Sensitive]]
[[TFHE]]
[[ThreadpoolComputeProcessor]]
[[ThresholdKeyshare]]
[[TrBFV]]
</details>
