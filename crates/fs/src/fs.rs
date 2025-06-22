use crate::traits::{DirectoryManager, FileCopier, FileFinder, FileReader, FileWriter, Replacer};
use anyhow::Result;
use futures::io::AsyncWriteExt;
use futures::stream::StreamExt;
use glob::Pattern;
use regex::Regex;
use std::path::Path;
use vfs::async_vfs::{AsyncMemoryFS, AsyncPhysicalFS, AsyncVfsPath};

pub struct Fs {
    root: AsyncVfsPath,
}

impl Fs {
    pub fn new(root: AsyncVfsPath) -> Self {
        Self { root }
    }

    pub fn mem() -> Self {
        let fs = AsyncMemoryFS::new();
        Self::new(fs.into())
    }

    pub fn physical_path<P: AsRef<Path>>(root_path: P) -> Result<Self> {
        let fs = AsyncPhysicalFS::new(root_path.as_ref());
        Ok(Self::new(fs.into()))
    }

    pub fn physical() -> Result<Self> {
        Ok(Self::physical_path("/")?)
    }
}

#[async_trait::async_trait]
impl FileReader for Fs {
    async fn read_to_string<P: AsRef<Path> + Send>(&self, path: P) -> Result<String> {
        let file_path = self.root.join(path.as_ref().to_string_lossy().as_ref())?;
        let contents = file_path.read_to_string().await?;
        Ok(contents)
    }
}

#[async_trait::async_trait]
impl FileWriter for Fs {
    async fn write_to_file<P: AsRef<Path> + Send>(&self, path: P, content: &str) -> Result<()> {
        let file_path = self.root.join(path.as_ref().to_string_lossy().as_ref())?;
        let mut file = file_path.create_file().await?;
        file.write_all(content.as_bytes()).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl DirectoryManager for Fs {
    async fn mkdirp<P: AsRef<Path> + Send>(&self, dest_path: P) -> Result<()> {
        let dir_path = self
            .root
            .join(dest_path.as_ref().to_string_lossy().as_ref())?;
        dir_path.create_dir_all().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl FileCopier for Fs {
    async fn cp<P1: AsRef<Path> + Send, P2: AsRef<Path> + Send>(
        &self,
        src_path: P1,
        dest_path: P2,
    ) -> Result<()> {
        let src_str = src_path.as_ref().to_string_lossy();
        let copy_contents_only = src_str.ends_with("/.");

        let actual_src = if copy_contents_only {
            let trimmed: &str = src_str.trim_end_matches("/.").as_ref();
            self.root.join(trimmed)?
        } else {
            self.root.join(src_str.as_ref())?
        };

        let dest = self
            .root
            .join(dest_path.as_ref().to_string_lossy().as_ref())?;

        if actual_src.is_file().await? {
            actual_src.copy_file(&dest).await?;
        } else if actual_src.is_dir().await? {
            if copy_contents_only {
                copy_dir_contents(&actual_src, &dest).await?;
            } else {
                actual_src.copy_dir(&dest).await?;
            }
        }

        Ok(())
    }
}

async fn copy_dir_contents(
    src_dir: &vfs::async_vfs::AsyncVfsPath,
    dest_dir: &vfs::async_vfs::AsyncVfsPath,
) -> Result<()> {
    if !dest_dir.exists().await? {
        dest_dir.create_dir_all().await?;
    }

    let entries = src_dir.read_dir().await?;
    let all_entries: Vec<_> = entries.collect().await;

    for entry in all_entries {
        let entry_name = entry.filename();
        let src_entry = src_dir.join(&entry_name)?;
        let dest_entry = dest_dir.join(&entry_name)?;

        if src_entry.is_file().await? {
            src_entry.copy_file(&dest_entry).await?;
        } else if src_entry.is_dir().await? {
            src_entry.copy_dir(&dest_entry).await?;
        }
    }

    Ok(())
}

#[async_trait::async_trait]
impl FileFinder for Fs {
    async fn find_files<P: AsRef<Path> + Send>(
        &self,
        in_folder: P,
        glob_pattern: &str,
    ) -> Result<Vec<String>> {
        let folder_path = self
            .root
            .join(in_folder.as_ref().to_string_lossy().as_ref())?;

        let pattern = Pattern::new(glob_pattern)?;
        let mut matching_files = Vec::new();

        println!("Folder Path: {}", folder_path.as_str());
        println!("Pattern: {pattern}");

        let all_entries: Vec<_> = folder_path
            .walk_dir()
            .await?
            .map(|res| res.unwrap())
            .collect::<Vec<_>>()
            .await;
        println!("ALL ENTRIES: {:?}", all_entries);
        for entry in all_entries {
            if entry.is_file().await? {
                let filename = entry.as_str();
                println!("entry is file for {filename}");
                if pattern.matches(filename) {
                    matching_files.push(entry.as_str().to_string());
                }
            }
        }
        Ok(matching_files)
    }
}

#[async_trait::async_trait]
impl Replacer for Fs {
    async fn replace_in_place<P: AsRef<Path> + Send + Sync>(
        &self,
        pattern: &Regex,
        replacement: &str,
        file_path: P,
    ) -> Result<()> {
        println!(
            "replace_in_place({:?},{:?},{:?})",
            &pattern,
            &replacement,
            file_path.as_ref()
        );
        let content = self.read_to_string(&file_path).await?;
        let new_content = pattern.replace_all(&content, replacement);
        if content != new_content {
            self.write_to_file(file_path, &new_content).await?;
        } else {
            println!("No change made to {:?}", file_path.as_ref());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn test_write_and_read_file() -> Result<()> {
        // Create an in-memory filesystem
        // let memory_fs = AsyncMemoryFS::new();
        let fs = Fs::mem();

        // Test data
        let test_path = "test_file.txt";
        let test_content = "Hello, World!\nThis is a test file.";

        // Write the file
        fs.write_to_file(test_path, test_content).await?;

        // Read the file back
        let read_content = fs.read_to_string(test_path).await?;

        // Verify the content matches
        assert_eq!(read_content, test_content);

        Ok(())
    }

    #[tokio::test]
    async fn test_mkdirp_creates_directory() -> Result<()> {
        // Create an in-memory filesystem
        let fs = Fs::mem();

        let test_dir = "some/deep/nested/directory";

        // Create the directory structure
        fs.mkdirp(test_dir).await?;

        // Verify the directory was created by checking if it exists
        let dir_path = fs.root.join(test_dir)?;
        assert!(dir_path.exists().await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_cp_recursive_copy() -> Result<()> {
        // Create an in-memory filesystem
        let fs = Fs::mem();

        // Set up source directory structure with files
        fs.mkdirp("src/subdir").await?;
        fs.write_to_file("src/file1.txt", "content of file1")
            .await?;
        fs.write_to_file("src/subdir/file2.txt", "content of file2")
            .await?;

        // Copy the entire directory structure
        fs.cp("src", "dest").await?;

        // Verify the destination structure was created
        let dest_path = fs.root.join("dest")?;
        assert!(dest_path.exists().await?);
        assert!(dest_path.is_dir().await?);

        // Verify files were copied
        let file1_content = fs.read_to_string("dest/file1.txt").await?;
        assert_eq!(file1_content, "content of file1");

        let file2_content = fs.read_to_string("dest/subdir/file2.txt").await?;
        assert_eq!(file2_content, "content of file2");

        Ok(())
    }

    #[tokio::test]
    async fn test_find_files_with_glob_pattern() -> Result<()> {
        // Create an in-memory filesystem
        let fs = Fs::mem();

        // Create a deeply nested folder structure with various files
        fs.mkdirp("project/src/utils").await?;
        fs.mkdirp("project/src/components").await?;
        fs.mkdirp("project/tests").await?;

        // Create files with different extensions
        fs.write_to_file("project/src/main.rs", "fn main() {}")
            .await?;
        fs.write_to_file("project/src/lib.rs", "pub mod utils;")
            .await?;
        fs.write_to_file("project/src/utils/helper.rs", "pub fn help() {}")
            .await?;
        fs.write_to_file("project/src/components/button.rs", "struct Button {}")
            .await?;
        fs.write_to_file("project/tests/integration.rs", "#[test] fn test() {}")
            .await?;
        fs.write_to_file("project/Cargo.toml", "[package]").await?;
        fs.write_to_file("project/README.md", "# Project").await?;

        // Find all .rs files
        let rust_files = fs.find_files("project", "*.rs").await?;

        // Verify we found all Rust files
        assert_eq!(rust_files.len(), 5);
        assert!(rust_files.contains(&"/project/src/main.rs".to_string()));
        assert!(rust_files.contains(&"/project/src/lib.rs".to_string()));
        assert!(rust_files.contains(&"/project/src/utils/helper.rs".to_string()));
        assert!(rust_files.contains(&"/project/src/components/button.rs".to_string()));
        assert!(rust_files.contains(&"/project/tests/integration.rs".to_string()));

        // Find files with specific pattern
        let toml_files = fs.find_files("project", "*.toml").await?;
        assert_eq!(toml_files.len(), 1);
        assert!(toml_files.contains(&"/project/Cargo.toml".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_replace_in_place() -> Result<()> {
        // Create an in-memory filesystem
        let fs = Fs::mem();

        // Create a test file with content to replace
        let test_path = "config.txt";
        let original_content =
            "server_url=localhost:8080\napi_version=v1\nserver_url=example.com:9090";
        fs.write_to_file(test_path, original_content).await?;

        // Create a regex pattern to replace all server_url values
        let pattern = Regex::new(r"server_url=([^\n]+)")?;
        let replacement = "server_url=production.example.com:443";

        // Apply the replacement
        fs.replace_in_place(&pattern, replacement, test_path)
            .await?;

        // Read the modified content
        let modified_content = fs.read_to_string(test_path).await?;

        // Verify the replacements were made
        let expected_content = "server_url=production.example.com:443\napi_version=v1\nserver_url=production.example.com:443";
        assert_eq!(modified_content, expected_content);

        Ok(())
    }
}
