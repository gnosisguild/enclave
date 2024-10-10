use std::{fs, io::Write, path::Path};

use tracing::{error, trace};
pub fn write_file_with_dirs(path: &str, content: &[u8]) -> std::io::Result<()> {
    let abs_path = if Path::new(path).is_absolute() {
        Path::new(path).to_path_buf()
    } else {
        let cwd = std::env::current_dir()?;
        cwd.join(path)
    };

    match abs_path.to_str() {
        Some(s) => trace!(path = s, "Writing to path"),
        None => error!(path=?abs_path, "Cannot parse path"),
    }

    // Ensure the directory structure exists
    if let Some(parent) = abs_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Open the file (creates it if it doesn't exist) and write the content
    let mut file = fs::File::create(&abs_path)?;
    file.write_all(content)?;
    trace!(
        path = abs_path.to_str().unwrap(),
        "File written successfully!"
    );
    Ok(())
}
