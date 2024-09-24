use clap::Parser;
use perf_rs::{Perf, Pmu};
use std::fs::File;

#[derive(Parser, Debug)]
#[clap(version)]
struct Args {
    #[clap(short, long, env)]
    runs: usize,

    #[clap(short, long, env)]
    out: String,

    #[clap(short, long, env)]
    bin: String,

    #[clap(short, long, value_parser, num_args = 1.., value_delimiter = ' ', allow_hyphen_values = true)]
    args: Vec<String>,
}

fn main() {
    let counters: usize;
    unsafe {
        let pmu = Pmu::cpuid();
        counters = pmu.num_counters as usize;
    }

    let args = Args::parse();
    let p = Perf::new(args.bin, args.args);
    let res = p.run(counters, args.runs);

    let file = File::create(args.out).unwrap();
    serde_json::to_writer(file, &res).unwrap();
}
