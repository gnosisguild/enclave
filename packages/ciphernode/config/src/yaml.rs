use anyhow::Result;
use std::{fs, path::PathBuf};

pub fn load_yaml_with_env(file_path: &PathBuf) -> Result<String> {
    // Read the file content to string
    let content = match fs::read_to_string(file_path) {
        Ok(val) => val,
        Err(_) => "".to_string()
    };

    // Collect environment variables and perform substitution
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
            "database:\n  url: $DATABASE_URL\n  password: ${{DB_PASSWORD}}"
        )?;

        // Set environment variables
        env::set_var("DATABASE_URL", "postgres://localhost:5432");
        env::set_var("DB_PASSWORD", "secret123");

        // Test the function
        let processed = load_yaml_with_env(&file_path)?;

        assert!(processed.contains("postgres://localhost:5432"));
        assert!(processed.contains("secret123"));

        Ok(())
    }
}
