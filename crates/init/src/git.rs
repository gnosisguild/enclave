use std::path::Path;

use anyhow::{Context, Result};
use tokio::process::Command;
use url::Url;

pub async fn shallow_clone(git_repo: &str, branch: &str, target_folder: &str) -> Result<()> {
    Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "--branch",
            branch,
            git_repo,
            target_folder,
        ])
        .status()
        .await?;
    Ok(())
}

pub async fn init(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    Command::new("git")
        .arg("init")
        .arg("-b")
        .arg("main")
        .current_dir(path)
        .output()
        .await
        .with_context(|| {
            format!(
                "Failed to execute git init in directory: {}",
                path.display()
            )
        })?;
    Ok(())
}

pub async fn add_all(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(path)
        .output()
        .await
        .with_context(|| format!("Failed to execute git add in directory: {}", path.display()))?;
    Ok(())
}

pub async fn commit(path: impl AsRef<Path>, message: &str) -> Result<()> {
    let path = path.as_ref();
    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(message)
        .current_dir(path)
        .output()
        .await
        .with_context(|| {
            format!(
                "Failed to execute git commit in directory: {}",
                path.display()
            )
        })?;
    Ok(())
}

pub async fn add_submodule(
    repo_path: impl AsRef<Path>,
    submodule_url: &str,
    submodule_path: &str,
) -> Result<()> {
    let repo_path = repo_path.as_ref();
    Command::new("git")
        .arg("submodule")
        .arg("add")
        .arg(submodule_url)
        .arg(submodule_path)
        .current_dir(repo_path)
        .output()
        .await
        .with_context(|| {
            format!(
                "Failed to add git submodule '{}' at '{}' in directory: {}",
                submodule_url,
                submodule_path,
                repo_path.display()
            )
        })?;
    Ok(())
}

#[derive(Debug)]
pub struct GitReference {
    pub base_url: String,
    pub branch: Option<String>,
    pub path: Option<String>,
}

pub fn parse_git_url(input: String) -> Result<GitReference, url::ParseError> {
    let url = Url::parse(&input)?;

    // Remove git+ prefix and fragment to get base URL
    let base_url = {
        let mut u = url.clone();
        u.set_fragment(None);
        u.to_string().trim_start_matches("git+").to_string()
    };

    // Parse fragment for branch:path
    let (branch, path) = if let Some(fragment) = url.fragment() {
        let parts: Vec<&str> = fragment.splitn(2, ':').collect();
        (
            Some(parts[0].to_string()),
            parts.get(1).map(|s| s.to_string()),
        )
    } else {
        (None, None)
    };

    Ok(GitReference {
        base_url,
        branch,
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_git_url;
    use anyhow::*;
    #[test]
    fn test_git_url() -> Result<()> {
        let g = parse_git_url(
            "git+https://github.com/gnosisguild/enclave.git#hacknet:template/default".to_string(),
        )?;

        assert_eq!(g.branch, Some("hacknet".to_string()));
        assert_eq!(
            g.base_url,
            "https://github.com/gnosisguild/enclave.git".to_string()
        );

        assert_eq!(g.path, Some("template/default".to_string()));

        Ok(())
    }
}
