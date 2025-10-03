use bytesize::ByteSize;
use std::{collections::HashMap, time::Duration};

pub struct SizeReporter {
    timers: HashMap<String, Duration>,
    sizes: HashMap<String, usize>,
}

impl SizeReporter {
    pub fn new() -> Self {
        Self {
            sizes: HashMap::new(),
            timers: HashMap::new(),
        }
    }

    pub fn log(&mut self, name: &str, bytes: &Vec<u8>) {
        self.sizes.insert(name.into(), bytes.len());
    }

    pub fn log_time(&mut self, name: &str, dur: Duration) {
        self.timers.insert(name.into(), dur);
    }

    pub fn to_size_table(&self) -> String {
        let entries: Vec<_> = self
            .sizes
            .iter()
            .map(|(name, &bytes)| (name.as_str(), ByteSize::b(bytes as u64).to_string()))
            .collect();
        self.format_table("Size", entries)
    }

    pub fn to_timing_table(&self) -> String {
        let entries: Vec<_> = self
            .timers
            .iter()
            .map(|(name, &dur)| (name.as_str(), format_duration(dur)))
            .collect();
        self.format_table("Duration", entries)
    }

    fn format_table(&self, column_header: &str, mut entries: Vec<(&str, String)>) -> String {
        if entries.is_empty() {
            return String::from("No data recorded");
        }

        entries.sort_by_key(|(name, _)| *name);

        let max_name_len = entries
            .iter()
            .map(|(name, _)| name.len())
            .max()
            .unwrap_or(4)
            .max(4);

        let mut output = String::new();

        // Header
        output.push_str(&format!(
            "{:<width$} | {}\n",
            "Name",
            column_header,
            width = max_name_len
        ));
        output.push_str(&format!(
            "{:-<width$}-+-{:-<10}\n",
            "",
            "",
            width = max_name_len
        ));

        // Data rows
        for (name, value) in entries {
            output.push_str(&format!(
                "{:<width$} | {}\n",
                name,
                value,
                width = max_name_len
            ));
        }

        output
    }
}

fn format_duration(dur: Duration) -> String {
    let micros = dur.as_micros();
    if micros < 1_000 {
        format!("{}µs", micros)
    } else if micros < 1_000_000 {
        format!("{:.2}ms", micros as f64 / 1_000.0)
    } else if micros < 60_000_000 {
        format!("{:.2}s", micros as f64 / 1_000_000.0)
    } else {
        format!("{:.2}m", micros as f64 / 60_000_000.0)
    }
}
