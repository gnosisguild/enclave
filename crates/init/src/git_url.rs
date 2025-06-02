use anyhow::{anyhow, Result};
use std::str::FromStr;

#[derive(Debug)]
pub struct GitUrl {
    pub repo_url: String,
    pub host: String,
    pub owner: String,
    pub repo: String,
    pub branch: String,
}

impl FromStr for GitUrl {
    type Err = anyhow::Error;

    fn from_str(url: &str) -> Result<Self> {
        let (base, branch) = url
            .split_once('#')
            .map(|(b, r)| (b, r.to_string()))
            .unwrap_or((url, "master".to_string()));

        let path = if base.contains("://") {
            base.split("://")
                .nth(1)
                .ok_or_else(|| anyhow!("invalid url"))?
        } else if base.starts_with("git@") {
            &base[4..]
        } else {
            return Err(anyhow!("unsupported format"));
        };

        let (host, repo_path) = if path.contains(':') && !path.contains('/') {
            path.split_once(':')
                .ok_or_else(|| anyhow!("invalid ssh format"))?
        } else {
            path.split_once('/')
                .ok_or_else(|| anyhow!("missing path"))?
        };

        let parts: Vec<&str> = repo_path.trim_end_matches(".git").split('/').collect();
        if parts.len() < 2 {
            return Err(anyhow!("invalid repo path"));
        }

        Ok(GitUrl {
            repo_url: base.to_string(),
            host: host.split('@').last().unwrap_or(host).to_string(),
            owner: parts[parts.len() - 2].to_string(),
            repo: parts[parts.len() - 1].to_string(),
            branch,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let cases = [
            (
                "https://github.com/user/repo.git#main",
                "https://github.com/user/repo.git",
                "github.com",
                "user",
                "repo",
                "main",
            ),
            (
                "git@github.com:user/repo.git",
                "git@github.com:user/repo.git",
                "github.com",
                "user",
                "repo",
                "master",
            ),
            (
                "ssh://git@gitlab.com/group/project.git#v1.0",
                "ssh://git@gitlab.com/group/project.git",
                "gitlab.com",
                "group",
                "project",
                "v1.0",
            ),
        ];

        for (url, repo_url, host, owner, repo, branch) in cases {
            let parsed: GitUrl = url.parse().unwrap();
            assert_eq!(parsed.repo_url, repo_url);
            assert_eq!(parsed.host, host);
            assert_eq!(parsed.owner, owner);
            assert_eq!(parsed.repo, repo);
            assert_eq!(parsed.branch, branch);
        }

        // Test the specific example
        let u: GitUrl = "https://github.com/foo/bar#v1.2.3".parse().unwrap();
        assert_eq!(u.repo_url, "https://github.com/foo/bar");
        assert_eq!(u.branch, "v1.2.3");

        // Test default branch
        let u: GitUrl = "https://github.com/foo/bar".parse().unwrap();
        assert_eq!(u.branch, "master");
    }
}
