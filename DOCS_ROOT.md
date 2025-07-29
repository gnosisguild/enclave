# Enclave Architecture Living Document

This is an Obsidian vault for the Enclave project architecture design. It is designed to help onboard and orient new team members to the project so that the structure and design decisions are understandable and accessible.

## Architecture

```mermaid
flowchart LR
	subgraph SDK
	    T["Typescript SDK"]
	    R["Rust SDK"]
	    N["Noir SDK"]
	end
    S["Support"]
    C["Ciphernode"]
    EVM["Contracts"]
    CLI["CLI"]

    C:::internal-link
    S:::internal-link
    EVM:::internal-link
    T:::internal-link
    R:::internal-link
    N:::internal-link
    CLI:::internal-link

    click C "http://github.com/gnosisguild/enclave/tree/main/crates/Ciphernode.md"
    click S "http://github.com/gnosisguild/enclave/tree/main/crates/support-scripts/Support.md"
    click EVM "http://github.com/gnosisguild/enclave/tree/main/packages/evm/docs/Contracts.md"
    click T "http://github.com/gnosisguild/enclave/tree/main/packages/enclave-sdk/Typescript SDK.md"
    click R "http://github.com/gnosisguild/enclave/tree/main/crates/sdk/Rust SDK.md"
    click N "http://github.com/gnosisguild/enclave/tree/main/architecture/Noir SDK.md"
    click CLI "http://github.com/gnosisguild/enclave/tree/main/crates/cli/CLI.md"
```
<details>
<summary>Links</summary>

[[CLI]]
[[Ciphernode]]
[[Contracts]]
[[Noir SDK]]
[[Rust SDK]]
[[Support]]
[[Typescript SDK]]
</details>

## Getting Started With Obsidian

### Prerequisites

- [Obsidian](https://obsidian.md/) installed on your system

### Opening the Vault

1. **Download/Clone the Repository**
2. **Open in Obsidian**

   - Launch Obsidian
   - Click "Open folder as vault"
   - Navigate to and select the main monorepo root.
   - Click "Open"

3. **Enable Required Plugin**
   - Once the vault is open you will be prompted to Trust the author of the repo. You can choose to do and have the plugin self install so or deny and install the Dataview plugin yourself. Inspect source code [here](https://github.com/blacksmithgu/obsidian-dataview)

### Using the Vault

This vault leverages Obsidian's linking and graph capabilities to create an interconnected view of the Enclave architecture. The Dataview plugin enables dynamic content generation and querying across documents, making it easier to find related information and maintain consistency across the documentation.
