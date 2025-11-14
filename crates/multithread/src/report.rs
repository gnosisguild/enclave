// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{collections::HashMap, thread, time::Duration};

use crate::TrackDuration;

#[derive(Default)]
pub struct MultithreadReport {
    rayon_threads: usize,
    max_simultaneous_rayon_tasks: usize,
    events: Vec<TrackDuration>,
}

impl MultithreadReport {
    pub fn new(rayon_threads: usize, max_simultaneous_rayon_tasks: usize) -> Self {
        Self {
            rayon_threads,
            max_simultaneous_rayon_tasks,
            events: Vec::new(),
        }
    }

    pub fn track(&mut self, msg: TrackDuration) {
        self.events.push(msg);
    }

    pub fn to_report(&self) -> FlattenedReport {
        let mut total_durations: HashMap<String, Duration> = HashMap::new();
        let mut runs: HashMap<String, u64> = HashMap::new();
        let cores_available: usize = match thread::available_parallelism() {
            Ok(count) => count.into(),
            Err(_) => 0usize,
        };

        // Accumulate durations and count runs
        for event in &self.events {
            *runs.entry(event.name.clone()).or_insert(0) += 1;

            total_durations
                .entry(event.name.clone())
                .and_modify(|d| *d += event.duration)
                .or_insert(event.duration);
        }

        // Calculate averages
        let avg_dur = total_durations
            .into_iter()
            .map(|(name, total)| {
                let count = runs[&name];
                let avg = Duration::from_nanos((total.as_nanos() / count as u128) as u64);
                (name, avg)
            })
            .collect();
        FlattenedReport {
            cores_available,
            avg_dur,
            rayon_threads: self.rayon_threads,
            max_simultaneous_rayon_tasks: self.max_simultaneous_rayon_tasks,
            runs,
        }
    }
}

pub struct FlattenedReport {
    cores_available: usize,
    rayon_threads: usize,
    max_simultaneous_rayon_tasks: usize,
    avg_dur: HashMap<String, Duration>,
    runs: HashMap<String, u64>,
}

impl std::fmt::Display for FlattenedReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:<32} {:>5}", "Rayon Threads:", self.rayon_threads)?;
        writeln!(
            f,
            "{:<32} {:>5}",
            "Max Simultaneous Rayon Tasks:", self.max_simultaneous_rayon_tasks
        )?;
        writeln!(f, "{:<32} {:>5}", "Cores Available:", self.cores_available)?;
        writeln!(f)?;
        writeln!(f, "{:<30} {:>15} {:>10}", "Name", "Avg Duration", "Runs")?;
        writeln!(f, "{}", "-".repeat(58))?;

        let mut entries: Vec<_> = self.avg_dur.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));

        for (name, avg_dur) in entries {
            let runs = self.runs.get(name).unwrap_or(&0);
            writeln!(f, "{:<30} {:>15?} {:>10}", name, avg_dur, runs)?;
        }

        Ok(())
    }
}
