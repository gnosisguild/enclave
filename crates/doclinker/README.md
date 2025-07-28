# Mermaid doclinker

This tool was quickly designed to link obsidian mermaid internal-link objects to github links to support architectural navigation on github.

It was specifically created in order to aid our documentation workflow.

Once you have edited one of the architectural document diagrams you can run the doclinker:

```bash
cargo install --path ./crates/doclinker --bin doclinker
```

Then run the tool to update all mermaid links

```bash
doclinker . https://github.com/gnosisguild/enclave
```
