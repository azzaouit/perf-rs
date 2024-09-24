use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use serde_aux::prelude::deserialize_number_from_string;
use std::arch::asm;
use std::collections::HashMap;
use std::process::Command;

/// PMU Details
#[derive(Debug)]
pub struct Pmu {
    /// Width of general-purpose PMCs
    pub pmc_width: u8,
    /// General-purpose PMCs per logical processor
    pub pmc_per_lp: u8,
    /// Version ID of PM architecture
    pub version: u8,
    /// Width of fixed counters
    pub counter_width: u8,
    /// Number of fixed counters
    pub num_counters: u8,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PerfEvent {
    event_name: Option<String>,
    metric_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct PerfCounter {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    counter_value: f64,
    event: String,
}

#[derive(Debug)]
pub struct Perf {
    exec: String,
    args: Vec<String>,
    events: Vec<String>,
}

impl Pmu {
    /// # Safety
    ///
    /// This function uses the cpuid instruction
    /// available on x86_64
    #[cfg(target_arch = "x86_64")]
    pub unsafe fn cpuid() -> Self {
        let mut edx: usize;
        let mut eax: usize = 10;
        asm!(
            "cpuid", inout("eax") eax, out("edx") edx
        );

        let num_counters: u8 = edx as u8 & 0x0f;
        edx >>= 5;
        let counter_width: u8 = edx as u8;

        let version: u8 = eax as u8;
        eax >>= 8;
        let pmc_per_lp: u8 = eax as u8;
        eax >>= 8;
        let pmc_width: u8 = eax as u8;

        Self {
            pmc_width,
            pmc_per_lp,
            version,
            counter_width,
            num_counters,
        }
    }
}

impl Perf {
    pub fn new(exec: String, args: Vec<String>) -> Self {
        let output = Command::new("perf")
            .arg("list")
            .arg("-j")
            .output()
            .expect("failed to launch perf");
        let json_str = String::from_utf8_lossy(&output.stdout);
        let pe: Vec<PerfEvent> = serde_json::from_str(&json_str).unwrap();
        let mut events = Vec::<String>::with_capacity(pe.len());
        for i in pe {
            match i.event_name {
                Some(x) => events.push(x),
                None => events.push(i.metric_name.unwrap()),
            }
        }
        Self { exec, args, events }
    }

    fn run_once(&self, counters: usize) -> Vec<PerfCounter> {
        let mut pc = Vec::<PerfCounter>::with_capacity(self.events.len());
        for events in self.events.chunks(counters) {
            let mut cmd = Command::new("perf");
            let out = cmd
                .arg("stat")
                .arg("-j")
                .arg("-e")
                .arg(events.join(","))
                .arg(&self.exec)
                .args(&self.args)
                .output()
                .expect("failed to run perf");
            let json_str = String::from_utf8_lossy(&out.stderr);
            for line in json_str.lines() {
                if let Ok(p) = serde_json::from_str(line) {
                    pc.push(p)
                }
            }
        }
        pc
    }

    pub fn run(&self, counters: usize, n: usize) -> HashMap<String, f64> {
        let mut pc = HashMap::<String, f64>::new();
        let pb = ProgressBar::new(n as u64);
        pb.set_style(ProgressStyle::default_bar().progress_chars("#> "));
        for _ in 0..n {
            let out = self.run_once(counters);
            for i in out.iter() {
                *pc.entry(i.event.clone()).or_insert(i.counter_value) += i.counter_value;
            }
            pb.inc(1);
        }
        for p in pc.iter_mut() {
            *p.1 /= n as f64;
        }
        pb.finish_and_clear();
        pc
    }
}
