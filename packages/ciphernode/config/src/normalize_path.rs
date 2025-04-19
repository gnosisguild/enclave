use std::{
    clone, env,
    path::{Path, PathBuf},
};

// Utility to normalize paths
// We use this so we can avoid using canonicalize() and having to have real files in order to
// manipulate and validate paths: https://doc.rust-lang.org/std/fs/fn.canonicalize.html
pub fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let path = expand_tilde(path.as_ref());

    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::Normal(name) => {
                components.push(name);
            }
            std::path::Component::RootDir => {
                components.clear();
                components.push(component.as_os_str());
            }
            std::path::Component::Prefix(prefix) => {
                components.push(prefix.as_os_str());
            }
            std::path::Component::CurDir => {}
        }
    }

    let mut result = PathBuf::new();
    for component in components {
        result.push(component);
    }
    result
}

pub fn relative_to(base: impl AsRef<Path>, path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        return PathBuf::from(path);
    } else {
        return PathBuf::from(base.as_ref().join(path));
    }
}

pub fn base_dir(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    PathBuf::from(path.parent().unwrap_or(path))
}

pub fn expand_tilde(path: &Path) -> PathBuf {
    let path_str = match path.to_str() {
        None => return path.to_path_buf(),
        Some(s) => s,
    };

    if !path_str.starts_with('~') {
        return path.to_path_buf();
    }

    let home_dir = match env::var("HOME") {
        Err(_) => return path.to_path_buf(),
        Ok(dir) => dir,
    };

    if path_str.len() == 1 {
        PathBuf::from(home_dir)
    } else {
        PathBuf::from(format!("{}{}", home_dir, &path_str[1..]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // Helper function to simplify test writing
    fn test_case(input: &str, expected: &str) {
        let normalized = normalize_path(Path::new(input));
        assert_eq!(normalized.to_string_lossy(), expected);
    }

    #[test]
    fn test_tilde_expansion() -> Result<(), String> {
        let expected = format!("{}/Documents", env::var("HOME").unwrap());
        test_case("~/Documents", &expected);
        Ok(())
    }

    #[test]
    fn test_parent_dir_resolution() {
        test_case("a/b/../c", "a/c");
    }

    #[test]
    fn test_multiple_parent_dirs() {
        test_case("a/../..", "");
    }

    #[test]
    fn test_cur_dir_ignores() {
        test_case("a/./b", "a/b");
    }

    #[test]
    fn test_root_dir_reset() {
        test_case("/a/../b", "/b");
    }

    #[test]
    fn test_over_parent_from_root() {
        test_case("/../..", "");
    }

    #[test]
    fn test_empty_path() {
        test_case("", "");
    }

    #[test]
    fn test_trailing_slash() {
        test_case("a/b/", "a/b");
    }

    #[test]
    fn test_root_edge_case() {
        test_case("/a/b/c/../..", "/a");
    }

    #[test]
    fn test_mixed_slashes() {
        test_case("a//b/../c", "a/c");
    }

    #[test]
    fn test_tilde_only() -> Result<(), String> {
        let expected = format!("{}", env::var("HOME").unwrap());
        test_case("~", &expected);
        Ok(())
    }

    #[test]
    fn test_absolute_path() {
        test_case("/home/user", "/home/user");
    }
}
