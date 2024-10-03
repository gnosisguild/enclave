use std::{fs,io::Write, path::Path};
pub fn write_file_with_dirs(path: &str, content: &[u8]) -> std::io::Result<()> {
    let abs_path = if Path::new(path).is_absolute() {
        Path::new(path).to_path_buf()
    } else {
        let cwd = std::env::current_dir()?;
        cwd.join(path)
    };

    // Ensure the directory structure exists
    if let Some(parent) = abs_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Open the file (creates it if it doesn't exist) and write the content
    let mut file = fs::File::create(&abs_path)?;
    file.write_all(content)?;

    println!("File written successfully: {:?}", abs_path);
    Ok(())
}
