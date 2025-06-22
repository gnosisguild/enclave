use anyhow::Result;
use e3_fs::prelude::*;
use e3_fs::Fs;
use std::path::Path;

use crate::copy::Filter;

pub async fn copy_with_filters_impl<P1, P2>(
    fs: &Fs,
    src_path: P1,
    dest_path: P2,
    filters: &[Filter],
) -> Result<()>
where
    P1: AsRef<Path> + Send + Sync,
    P2: AsRef<Path> + Send + Sync,
{
    println!("src:{:?}\ndest:{:?}", src_path.as_ref(), dest_path.as_ref());
    fs.mkdirp(&dest_path).await?;
    fs.cp(&src_path, &dest_path).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::copy::Filter;
    use anyhow::Result;
    use e3_fs::Fs;

    #[tokio::test]
    async fn test_copy_with_filters_impl() -> Result<()> {
        // Create an in-memory filesystem
        let fs = Fs::mem();

        // Define source and destination paths
        let src_path = "/tmp/mysource/.";
        let dest_path = "/home/user/myproj";

        fs.mkdirp(&dest_path).await?;

        // Create some test files in the source directory
        fs.mkdirp(&src_path).await?;
        fs.write_to_file(format!("{}/file1.txt", src_path), "Content of file 1")
            .await?;
        fs.write_to_file(format!("{}/file2.txt", src_path), "Content of file 2")
            .await?;
        fs.mkdirp(format!("{}/subdir", src_path)).await?;
        fs.write_to_file(
            format!("{}/subdir/file3.txt", src_path),
            "Content of file 3",
        )
        .await?;

        // Create empty filters for this test
        let filters: Vec<Filter> = vec![];

        // Execute the copy operation
        copy_with_filters_impl(&fs, &src_path, &dest_path, &filters).await?;
        // Verify that files were copied to the destination
        // Check that the original files exist in source
        assert!(fs
            .read_to_string(format!("{}/file1.txt", src_path))
            .await
            .is_ok());
        assert!(fs
            .read_to_string(format!("{}/file2.txt", src_path))
            .await
            .is_ok());
        assert!(fs
            .read_to_string(format!("{}/subdir/file3.txt", src_path))
            .await
            .is_ok());

        // Check that files were copied to destination
        let copied_file1 = fs
            .read_to_string(format!("{}/file1.txt", dest_path))
            .await?;
        let copied_file2 = fs
            .read_to_string(format!("{}/file2.txt", dest_path))
            .await?;
        let copied_file3 = fs
            .read_to_string(format!("{}/subdir/file3.txt", dest_path))
            .await?;

        // Verify content matches
        assert_eq!(copied_file1, "Content of file 1");
        assert_eq!(copied_file2, "Content of file 2");
        assert_eq!(copied_file3, "Content of file 3");

        // Additional verification: check that the source files still exist (copy, not move)
        let original_file1 = fs.read_to_string(format!("{}/file1.txt", src_path)).await?;
        let original_file2 = fs.read_to_string(format!("{}/file2.txt", src_path)).await?;
        let original_file3 = fs
            .read_to_string(format!("{}/subdir/file3.txt", src_path))
            .await?;

        assert_eq!(original_file1, "Content of file 1");
        assert_eq!(original_file2, "Content of file 2");
        assert_eq!(original_file3, "Content of file 3");

        Ok(())
    }
}
