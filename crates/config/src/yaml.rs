use anyhow::Result;
use std::{fs, path::PathBuf};

pub fn load_yaml_with_env(file_path: &PathBuf) -> Result<String> {
    let content = fs::read_to_string(file_path)?;
    Ok(shellexpand::env(&content)?.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_yaml_env_substitution() -> Result<()> {
        // Create a temporary directory and file
        let dir = tempdir()?;
        let file_path = dir.path().join("test.yaml");
        let mut file = File::create(&file_path)?;

        // Write test YAML content
        writeln!(
            file,
            "database:\n  url: $MY_DATABASE_URL\n  password: ${{MY_DB_PASSWORD}}"
        )?;

        // Set environment variables
        env::set_var("MY_DATABASE_URL", "postgres://localhost:5432");
        env::set_var("MY_DB_PASSWORD", "secret123");

        // Test the function
        let processed = load_yaml_with_env(&file_path)?;

        env::remove_var("MY_DATABASE_URL");
        env::remove_var("MY_DB_PASSWORD");

        assert!(processed.contains("postgres://localhost:5432"));
        assert!(processed.contains("secret123"));

        Ok(())
    }
}
