use clap::{Arg, Command};
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
struct MermaidDiagram {
    full_match: String,
    content: String,
    start_index: usize,
    end_index: usize,
}

struct MermaidProcessor {
    root_folder: PathBuf,
    base_github_url: String,
    document_index: HashMap<String, PathBuf>,
}

impl MermaidProcessor {
    fn new(root_folder: PathBuf, base_github_url: String) -> Self {
        Self {
            root_folder,
            base_github_url,
            document_index: HashMap::new(),
        }
    }

    /// Build an index of all markdown documents for quick lookup
    fn build_document_index(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Building document index...");

        for entry in WalkDir::new(&self.root_folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
        {
            let file_path = entry.path();
            let relative_path = file_path.strip_prefix(&self.root_folder)?;

            if let Some(file_stem) = file_path.file_stem() {
                if let Some(doc_name) = file_stem.to_str() {
                    // Index with original name
                    self.document_index
                        .insert(doc_name.to_string(), relative_path.to_path_buf());

                    // Index with spaces instead of hyphens/underscores
                    let spaced_name = doc_name.replace(['-', '_'], " ");
                    self.document_index
                        .insert(spaced_name.clone(), relative_path.to_path_buf());

                    // Index lowercase versions
                    self.document_index
                        .insert(doc_name.to_lowercase(), relative_path.to_path_buf());
                    self.document_index
                        .insert(spaced_name.to_lowercase(), relative_path.to_path_buf());
                }
            }
        }

        println!("Found {} documents", self.document_index.len() / 4); // Divided by 4 due to multiple entries per file
        Ok(())
    }

    /// Find document path based on node label using fuzzy matching
    fn find_document_path(&self, node_label: &str) -> Option<&PathBuf> {
        // Clean the node label (remove quotes, brackets, extra whitespace)
        let clean_label = node_label.trim().trim_matches(['"', '[', ']']).trim();

        // Try exact match first
        if let Some(path) = self.document_index.get(clean_label) {
            return Some(path);
        }

        // Try case-insensitive match
        if let Some(path) = self.document_index.get(&clean_label.to_lowercase()) {
            return Some(path);
        }

        // Try partial matches
        for (doc_name, doc_path) in &self.document_index {
            let doc_lower = doc_name.to_lowercase();
            let label_lower = clean_label.to_lowercase();

            if doc_lower.contains(&label_lower) || label_lower.contains(&doc_lower) {
                return Some(doc_path);
            }
        }

        None
    }

    /// Extract all mermaid diagrams from markdown content
    fn extract_mermaid_diagrams(&self, content: &str) -> Vec<MermaidDiagram> {
        let re = Regex::new(r"```mermaid\s*([\s\S]*?)```").unwrap();
        let mut diagrams = Vec::new();

        for cap in re.captures_iter(content) {
            if let Some(full_match) = cap.get(0) {
                if let Some(diagram_content) = cap.get(1) {
                    diagrams.push(MermaidDiagram {
                        full_match: full_match.as_str().to_string(),
                        content: diagram_content.as_str().trim().to_string(),
                        start_index: full_match.start(),
                        end_index: full_match.end(),
                    });
                }
            }
        }

        diagrams
    }

    /// Parse mermaid diagram to extract node definitions and their labels
    fn parse_mermaid_nodes(&self, mermaid_content: &str) -> HashMap<String, String> {
        let mut nodes = HashMap::new();

        // Regex patterns for different node definition styles
        let node_def_re = Regex::new(r"(\w+)\s*\[([^\]]+)\]").unwrap();
        let connection_re = Regex::new(r"(\w+)\s*[-=]+>?\s*(\w+)").unwrap();

        for line in mermaid_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }

            // Check for node definitions with labels: nodeId["label"] or nodeId[label]
            if let Some(caps) = node_def_re.captures(trimmed) {
                if let (Some(node_id), Some(label)) = (caps.get(1), caps.get(2)) {
                    nodes.insert(node_id.as_str().to_string(), label.as_str().to_string());
                }
                continue;
            }

            // Check for simple connections to capture node IDs: nodeA --> nodeB
            if let Some(caps) = connection_re.captures(trimmed) {
                if let (Some(node_a), Some(node_b)) = (caps.get(1), caps.get(2)) {
                    let node_a_str = node_a.as_str().to_string();
                    let node_b_str = node_b.as_str().to_string();

                    nodes.entry(node_a_str.clone()).or_insert(node_a_str);
                    nodes.entry(node_b_str.clone()).or_insert(node_b_str);
                }
            }
        }

        nodes
    }

    /// Find all nodes that have the internal-link class
    fn find_internal_link_nodes(&self, mermaid_content: &str) -> Vec<String> {
        let re = Regex::new(r"(\w+):::internal-link").unwrap();
        let mut internal_link_nodes = Vec::new();

        for cap in re.captures_iter(mermaid_content) {
            if let Some(node_id) = cap.get(1) {
                internal_link_nodes.push(node_id.as_str().to_string());
            }
        }

        internal_link_nodes
    }

    /// Generate click handlers for internal link nodes
    fn generate_click_handlers(
        &self,
        internal_link_nodes: &[String],
        all_nodes: &HashMap<String, String>,
    ) -> Vec<String> {
        let mut click_handlers = Vec::new();

        for node_id in internal_link_nodes {
            let node_label = all_nodes.get(node_id).unwrap_or(node_id);

            if let Some(doc_path) = self.find_document_path(node_label) {
                let doc_path_str = doc_path.to_string_lossy().replace('\\', "/");
                let github_url = format!("{}/{}", self.base_github_url, doc_path_str);
                click_handlers.push(format!("    click {} \"{}\"", node_id, github_url));
            } else {
                eprintln!(
                    "Warning: Could not find document for node \"{}\" (label: \"{}\")",
                    node_id, node_label
                );
                // Still add a placeholder click handler
                let github_url = format!("{}/path/to/{}.md", self.base_github_url, node_label);
                click_handlers.push(format!("    click {} \"{}\"", node_id, github_url));
            }
        }

        click_handlers
    }

    /// Remove existing click handlers from mermaid content
    fn remove_existing_click_handlers(&self, mermaid_content: &str) -> String {
        mermaid_content
            .lines()
            .filter(|line| !line.trim().starts_with("click "))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Process a single mermaid diagram
    fn process_mermaid_diagram(&self, mermaid_content: &str) -> String {
        // Remove existing click handlers
        let clean_content = self.remove_existing_click_handlers(mermaid_content);

        // Find all nodes and internal-link nodes
        let all_nodes = self.parse_mermaid_nodes(&clean_content);
        let internal_link_nodes = self.find_internal_link_nodes(&clean_content);

        if internal_link_nodes.is_empty() {
            return mermaid_content.to_string(); // No changes needed
        }

        // Generate click handlers
        let click_handlers = self.generate_click_handlers(&internal_link_nodes, &all_nodes);

        if click_handlers.is_empty() {
            return mermaid_content.to_string(); // No valid click handlers to add
        }

        // Add click handlers at the end
        format!(
            "{}\n\n{}",
            clean_content.trim_end(),
            click_handlers.join("\n")
        )
    }

    /// Process a single markdown file
    fn process_markdown_file(&self, file_path: &Path) -> Result<(), Box<dyn Error>> {
        println!("Processing: {}", file_path.display());

        let content = fs::read_to_string(file_path)?;
        let diagrams = self.extract_mermaid_diagrams(&content);

        if diagrams.is_empty() {
            return Ok(()); // No mermaid diagrams found
        }

        let mut modified_content = content.clone();
        let mut offset: i32 = 0;

        for diagram in &diagrams {
            let processed_diagram = self.process_mermaid_diagram(&diagram.content);

            if processed_diagram != diagram.content {
                let new_full_diagram = format!("```mermaid\n{}\n```", processed_diagram);
                let start_pos = (diagram.start_index as i32 + offset) as usize;
                let end_pos = (diagram.end_index as i32 + offset) as usize;

                modified_content.replace_range(start_pos..end_pos, &new_full_diagram);

                offset += new_full_diagram.len() as i32 - diagram.full_match.len() as i32;
                println!("  Updated mermaid diagram in {}", file_path.display());
            }
        }

        if modified_content != content {
            fs::write(file_path, modified_content)?;
            println!("  Saved changes to {}", file_path.display());
        }

        Ok(())
    }

    /// Process all markdown files recursively
    fn process_markdown_files(&self) -> Result<(), Box<dyn Error>> {
        println!("Processing markdown files...");

        for entry in WalkDir::new(&self.root_folder)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
        {
            if let Err(e) = self.process_markdown_file(entry.path()) {
                eprintln!("Error processing {}: {}", entry.path().display(), e);
            }
        }

        Ok(())
    }

    /// Main processing function
    pub fn process(&mut self) -> Result<(), Box<dyn Error>> {
        self.build_document_index()?;
        self.process_markdown_files()?;
        println!("Processing complete!");
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("Mermaid Internal Link Processor")
        .version("1.0")
        .author("Enclave")
        .about("Processes mermaid diagrams in markdown files to add GitHub click handlers for internal-link nodes")
        .arg(
            Arg::new("folder")
                .help("The folder path containing markdown files")
                .required(true)
                .index(1)
        )
        .arg(
            Arg::new("github-url")
                .help("The base GitHub URL (eg: https://github.com/user/repo)")
                .required(true)
                .index(2)
        )
        .get_matches();

    let folder_path = matches.get_one::<String>("folder").unwrap();
    let github_base_url = matches.get_one::<String>("github-url").unwrap();
    let github_base_url = [github_base_url, "tree/main"].join("/");

    let path = Path::new(folder_path);
    if !path.exists() {
        eprintln!(
            "Error: Folder '{}' does not exist or is not accessible.",
            folder_path
        );
        std::process::exit(1);
    }

    let mut processor = MermaidProcessor::new(path.to_path_buf(), github_base_url.to_string());

    processor.process()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extract_mermaid_diagrams() {
        let processor = MermaidProcessor::new(PathBuf::new(), String::new());
        let content = r#"
# Test Document

Some text here.

```mermaid
flowchart TB
    A --> B
    B --> C
```

More text.

```mermaid
graph LR
    X --> Y
```

Final text.
"#;

        let diagrams = processor.extract_mermaid_diagrams(content);
        assert_eq!(diagrams.len(), 2);
        assert!(diagrams[0].content.contains("flowchart TB"));
        assert!(diagrams[0].content.contains("A --> B"));
        assert!(diagrams[1].content.contains("graph LR"));
        assert!(diagrams[1].content.contains("X --> Y"));

        // Test that indices are correct
        assert!(diagrams[0].start_index < diagrams[0].end_index);
        assert!(diagrams[1].start_index < diagrams[1].end_index);
        assert!(diagrams[0].end_index < diagrams[1].start_index);
    }

    #[test]
    fn test_extract_mermaid_diagrams_empty_content() {
        let processor = MermaidProcessor::new(PathBuf::new(), String::new());
        let content = "# No mermaid diagrams here\n\nJust regular markdown.";

        let diagrams = processor.extract_mermaid_diagrams(content);
        assert_eq!(diagrams.len(), 0);
    }

    #[test]
    fn test_find_internal_link_nodes() {
        let processor = MermaidProcessor::new(PathBuf::new(), String::new());
        let mermaid_content = r#"
flowchart TB
    A --> B
    B --> C
    D --> E
    
    A:::internal-link
    C:::internal-link
    D:::some-other-class
"#;

        let nodes = processor.find_internal_link_nodes(mermaid_content);
        assert_eq!(nodes.len(), 2);
        assert!(nodes.contains(&"A".to_string()));
        assert!(nodes.contains(&"C".to_string()));
        assert!(!nodes.contains(&"B".to_string()));
        assert!(!nodes.contains(&"D".to_string()));
    }

    #[test]
    fn test_find_internal_link_nodes_empty() {
        let processor = MermaidProcessor::new(PathBuf::new(), String::new());
        let mermaid_content = r#"
flowchart TB
    A --> B
    B --> C
    
    A:::regular-class
    C:::another-class
"#;

        let nodes = processor.find_internal_link_nodes(mermaid_content);
        assert_eq!(nodes.len(), 0);
    }

    #[test]
    fn test_parse_mermaid_nodes() {
        let processor = MermaidProcessor::new(PathBuf::new(), String::new());
        let mermaid_content = r#"
flowchart TB
    S["Support"]
    C["Ciphernode"]
    EVM["Contracts"]
    T[Typescript SDK]
    CLI --> S
    CLI --> C
    C --> EVM
    T --> EVM
"#;

        let nodes = processor.parse_mermaid_nodes(mermaid_content);
        assert_eq!(nodes.get("S"), Some(&"\"Support\"".to_string()));
        assert_eq!(nodes.get("C"), Some(&"\"Ciphernode\"".to_string()));
        assert_eq!(nodes.get("EVM"), Some(&"\"Contracts\"".to_string()));
        assert_eq!(nodes.get("T"), Some(&"Typescript SDK".to_string()));
        assert!(nodes.contains_key("CLI"));

        // CLI should have been added from connections
        assert_eq!(nodes.get("CLI"), Some(&"CLI".to_string()));
    }

    #[test]
    fn test_remove_existing_click_handlers() {
        let processor = MermaidProcessor::new(PathBuf::new(), String::new());
        let mermaid_content = r#"flowchart TB
    A --> B
    B --> C
    
    A:::internal-link
    C:::internal-link
    
    click A "https://old-url.com/a"
    click C "https://old-url.com/c""#;

        let cleaned = processor.remove_existing_click_handlers(mermaid_content);
        assert!(!cleaned.contains("click A"));
        assert!(!cleaned.contains("click C"));
        assert!(cleaned.contains("A --> B"));
        assert!(cleaned.contains("A:::internal-link"));
    }

    #[test]
    fn test_document_index_and_path_finding() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create test markdown files
        fs::create_dir_all(root_path.join("docs")).unwrap();
        fs::write(root_path.join("support.md"), "# Support Document").unwrap();
        fs::write(
            root_path.join("docs/ciphernode.md"),
            "# Ciphernode Document",
        )
        .unwrap();
        fs::write(root_path.join("docs/typescript-sdk.md"), "# TypeScript SDK").unwrap();
        fs::write(root_path.join("contracts.md"), "# Smart Contracts").unwrap();

        let mut processor = MermaidProcessor::new(
            root_path.to_path_buf(),
            "https://github.com/test/repo".to_string(),
        );

        processor.build_document_index().unwrap();

        // Test exact matches
        assert!(processor.find_document_path("support").is_some());
        assert!(processor.find_document_path("ciphernode").is_some());
        assert!(processor.find_document_path("typescript-sdk").is_some());

        // Test case-insensitive matches
        assert!(processor.find_document_path("Support").is_some());
        assert!(processor.find_document_path("CIPHERNODE").is_some());

        // Test spaced names
        assert!(processor.find_document_path("typescript sdk").is_some());

        // Test partial matches
        assert!(processor.find_document_path("Contracts").is_some());

        // Test non-existent
        assert!(processor.find_document_path("nonexistent").is_none());
    }

    #[test]
    fn test_generate_click_handlers() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create test files
        fs::write(root_path.join("support.md"), "# Support").unwrap();
        fs::write(root_path.join("ciphernode.md"), "# Ciphernode").unwrap();

        let mut processor = MermaidProcessor::new(
            root_path.to_path_buf(),
            "https://github.com/test/repo".to_string(),
        );
        processor.build_document_index().unwrap();

        let internal_link_nodes = vec!["S".to_string(), "C".to_string(), "MISSING".to_string()];
        let mut all_nodes = HashMap::new();
        all_nodes.insert("S".to_string(), "Support".to_string());
        all_nodes.insert("C".to_string(), "Ciphernode".to_string());
        all_nodes.insert("MISSING".to_string(), "NonExistent".to_string());

        let click_handlers = processor.generate_click_handlers(&internal_link_nodes, &all_nodes);

        assert_eq!(click_handlers.len(), 3);
        assert!(click_handlers[0].contains("click S \"https://github.com/test/repo/support.md\""));
        assert!(
            click_handlers[1].contains("click C \"https://github.com/test/repo/ciphernode.md\"")
        );
        assert!(click_handlers[2]
            .contains("click MISSING \"https://github.com/test/repo/path/to/NonExistent.md\""));
    }

    #[test]
    fn test_process_mermaid_diagram_complete() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create test files
        fs::write(root_path.join("support.md"), "# Support").unwrap();
        fs::write(root_path.join("ciphernode.md"), "# Ciphernode").unwrap();

        let mut processor = MermaidProcessor::new(
            root_path.to_path_buf(),
            "https://github.com/test/repo".to_string(),
        );
        processor.build_document_index().unwrap();

        let mermaid_content = r#"flowchart TB
    S["Support"]
    C["Ciphernode"]
    CLI["CLI"]

    CLI --> S
    CLI --> C

    S:::internal-link
    C:::internal-link"#;

        let processed = processor.process_mermaid_diagram(mermaid_content);

        assert!(processed.contains("flowchart TB"));
        assert!(processed.contains("CLI --> S"));
        assert!(processed.contains("S:::internal-link"));
        assert!(processed.contains("click S \"https://github.com/test/repo/support.md\""));
        assert!(processed.contains("click C \"https://github.com/test/repo/ciphernode.md\""));
    }

    #[test]
    fn test_process_mermaid_diagram_no_internal_links() {
        let processor = MermaidProcessor::new(PathBuf::new(), String::new());

        let mermaid_content = r#"flowchart TB
    A --> B
    B --> C"#;

        let processed = processor.process_mermaid_diagram(mermaid_content);
        assert_eq!(processed, mermaid_content);
    }

    #[test]
    fn test_full_markdown_file_processing() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create supporting documents
        fs::write(root_path.join("support.md"), "# Support Document").unwrap();
        fs::write(root_path.join("ciphernode.md"), "# Ciphernode Document").unwrap();

        // Create main markdown file with mermaid diagram
        let markdown_content = r#"# Architecture Overview

This document shows our system architecture.

```mermaid
flowchart TB
    S["Support"]
    C["Ciphernode"]
    CLI["CLI"]

    CLI --> S
    CLI --> C

    S:::internal-link
    C:::internal-link
```

That's our architecture!
"#;

        let test_file = root_path.join("architecture.md");
        fs::write(&test_file, markdown_content).unwrap();

        let mut processor = MermaidProcessor::new(
            root_path.to_path_buf(),
            "https://github.com/test/repo".to_string(),
        );
        processor.build_document_index().unwrap();

        // Process the file
        processor.process_markdown_file(&test_file).unwrap();

        // Read the modified content
        let modified_content = fs::read_to_string(&test_file).unwrap();

        // Verify the original structure is preserved
        assert!(modified_content.contains("# Architecture Overview"));
        assert!(modified_content.contains("This document shows"));
        assert!(modified_content.contains("That's our architecture!"));

        // Verify the mermaid diagram was updated
        assert!(modified_content.contains("```mermaid"));
        assert!(modified_content.contains("flowchart TB"));
        assert!(modified_content.contains("CLI --> S"));
        assert!(modified_content.contains("S:::internal-link"));

        // Verify click handlers were added
        assert!(modified_content.contains("click S \"https://github.com/test/repo/support.md\""));
        assert!(modified_content.contains("click C \"https://github.com/test/repo/ciphernode.md\""));
    }

    #[test]
    fn test_multiple_mermaid_diagrams_in_file() {
        let temp_dir = TempDir::new().unwrap();
        let root_path = temp_dir.path();

        // Create supporting documents
        fs::write(root_path.join("api.md"), "# API").unwrap();
        fs::write(root_path.join("database.md"), "# Database").unwrap();

        let markdown_content = r#"# System Overview

## Frontend Flow
```mermaid
graph TD
    UI --> API
    API:::internal-link
```

## Backend Flow
```mermaid
flowchart LR
    API --> DB["Database"]
    DB:::internal-link
```
"#;

        let test_file = root_path.join("overview.md");
        fs::write(&test_file, markdown_content).unwrap();

        let mut processor = MermaidProcessor::new(
            root_path.to_path_buf(),
            "https://github.com/test/repo".to_string(),
        );
        processor.build_document_index().unwrap();
        processor.process_markdown_file(&test_file).unwrap();

        let modified_content = fs::read_to_string(&test_file).unwrap();

        // Both diagrams should have click handlers
        assert!(modified_content.contains("click API \"https://github.com/test/repo/api.md\""));
        assert!(modified_content.contains("click DB \"https://github.com/test/repo/database.md\""));

        // Count the number of click handlers (should be 2)
        let click_count = modified_content.matches("click ").count();
        assert_eq!(click_count, 2);
    }
}
