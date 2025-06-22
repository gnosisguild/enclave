use anyhow::Result;
use regex::Regex;
use std::path::Path;

#[async_trait::async_trait]
pub trait FileReader {
    async fn read_to_string<P: AsRef<Path> + Send + Sync>(&self, path: P) -> Result<String>;
}

#[async_trait::async_trait]
pub trait FileWriter {
    async fn write_to_file<P: AsRef<Path> + Send>(&self, path: P, content: &str) -> Result<()>;
}

#[async_trait::async_trait]
pub trait DirectoryManager {
    async fn mkdirp<P: AsRef<Path> + Send>(&self, dest_path: P) -> Result<()>;
}

#[async_trait::async_trait]
pub trait FileCopier {
    async fn cp<P1: AsRef<Path> + Send, P2: AsRef<Path> + Send>(
        &self,
        src_path: P1,
        dest_path: P2,
    ) -> Result<()>;
}

#[async_trait::async_trait]
pub trait FileFinder {
    async fn find_files<P: AsRef<Path> + Send>(
        &self,
        in_folder: P,
        glob_pattern: &str,
    ) -> Result<Vec<String>>;
}

#[async_trait::async_trait]
pub trait Replacer {
    async fn replace_in_place<P: AsRef<Path> + Send + Sync>(
        &self,
        pattern: &Regex,
        replacement: &str,
        file_path: P,
    ) -> Result<()>;
}
