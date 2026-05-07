// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Generator CLI for ML training instances.
//!
//! Synthesizes random `ExactCoverInput`s and runs the solver with branch tracing.
//! Successful solves emit JSONL records to a single output file.

use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicI32;

use clap::Parser;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

use maplestory_union_solver_wasm::{
    CancelFlag, PolyominoCatalog, SolveOptions, Tracer, UnionBoard,
    build_instance, solve_exact_cover
};

#[derive(Parser, Debug)]
#[command(version, about = "Synthesize ML training instances", long_about = None)]
struct Args {
    /// Number of instances to synthesize.
    #[arg(short, long, default_value_t = 100)]
    count: usize,

    /// Random seed. If omitted, draws from system entropy each run.
    #[arg(short, long, default_value_t = 42)]
    seed: u64,

    /// Per-instance solver timeout, in seconds.
    #[arg(short, long)]
    timeout: Option<u64>,

    /// Luby restart base in nodes.
    #[arg(short = 'b', long, default_value_t = 100_000)]
    luby_base: u64,

    /// Number of parallel workers.
    /// 0 = auto (available_parallelism - 1.)
    /// N > 0 = N workers race with independent seeds.
    #[arg(short, long, default_value_t = 0)]
    workers: usize,

    /// Number of solver runs. Each run uses a fresh seed
    /// (unless --seed is set, in which case the same seed is reused).
    #[arg(short, long, default_value_t = 1)]
    runs: u32,

    /// Output JSONL file path.
    #[arg(short, long)]
    out: PathBuf,

    /// Suppress per-instance progress output.
    #[arg(short, long)]
    quiet: bool
}

pub fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    let board = UnionBoard::new();
    let catalog = PolyominoCatalog::enumerate(6);

    let file = File::create(&args.out)?;
    let writer = BufWriter::new(file);
    let mut tracer = Tracer::new(Box::new(writer));

    let mut rng = Xoshiro256PlusPlus::seed_from_u64(args.seed);

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for i in 0..args.count {
        let input = build_instance(&catalog, &board, &mut rng);
        let instance_id = format!("synth_{i:08}");
        tracer.set_instance_id(instance_id.clone());

        let solver_seed = rng.next_u64();
        let opts = SolveOptions {
            seed: Some(solver_seed),
            timeout_ms: args.timeout.map(|s| s * 1000),
            luby_base: args.luby_base
        };

        // Cancel atom required by signature;
        // not actively used in single-threaded mode.
        let cancel_atom = AtomicI32::new(0);
        let cancel = CancelFlag::new(&cancel_atom);

        match solve_exact_cover(&input, opts, Some(&cancel), Some(&mut tracer)) {
            Ok(r) => {
                if r.solution.is_some() {
                    succeeded += 1;
                    if !args.quiet {
                        println!(
                            "[{:>4}/{:<4}] {} solved in {}ms ({} nodes)",
                            i + 1, args.count, instance_id,
                            r.stats.common.elapsed_ms, r.stats.common.node_count
                        );
                    }
                } else {
                    failed += 1;
                    if !args.quiet {
                        println!(
                            "[{:>4}/{:<4}] {} timed out or no solution",
                            i + 1, args.count, instance_id
                        );
                    }
                }
            }

            Err(e) => {
                eprintln!("[{}/{}] {} error: {}", i + 1, args.count, instance_id, e);
                failed += 1;
            }
        }
    }

    println!("\nDone. {} succeeded, {} failed.", succeeded, failed);
    Ok(())
}