use env_logger::{Builder, Target};
use log::{LevelFilter, Record};
use std::io::Write;
use std::path::Path;
use chrono::Utc;

pub fn init_logger() {
    let mut builder = Builder::new();
    builder
        .target(Target::Stdout)
        .filter(None, LevelFilter::Info)
        .format(|buf, record: &Record| {
            let file = record.file().unwrap_or("unknown");
            let filename = Path::new(file).file_name().unwrap_or_else(|| file.as_ref());
            let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S");

            writeln!(
                buf,
                "[{}] [{}:{}] - {}",
                timestamp,
                filename.to_string_lossy(),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .init();
}