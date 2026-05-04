// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Benchmark CLI for the ExactCover solver.
//!
//! Reads a JSON input file, runs the solver one or more times, and reports timing statistics.
//! Supports writing the JSON result to a file.

use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use clap::Parser;

use maplestory_union_solver_wasm::{
    CancelFlag, ExactCoverInput, ExactCoverResult, SolveOptions, solve_exact_cover
};

fn auto_workers() -> usize {
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .saturating_sub(1)
        .max(2)
}

#[derive(Parser, Debug)]
#[command(version, about = "MapleStory Union solver benchmark", long_about = None)]
struct Args {
    /// JSON input file path.
    input: PathBuf,

    /// Random seed. If omitted, draws from system entropy each run.
    #[arg(short, long)]
    seed: Option<u64>,

    /// Wall-clock timeout in seconds. No timeout if omitted.
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

    /// Write the last run's JSON result to this file.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Suppress per-run progress output.
    #[arg(short, long)]
    quiet: bool,

    /// Print JSON result to stdout (last run) instead of human summary.
    #[arg(short, long)]
    json: bool
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Stats collected from a single worker inside a portfolio race.
struct WorkerStats {
    worker_idx: usize,
    result: ExactCoverResult
}

/// Outcome of one worker race (or single-worker run).
struct RunOutcome {
    /// Wall-clock elapsed for the whole race.
    elapsed_ms: u64,

    /// The winning result.
    winner: Option<WorkerStats>,

    /// Sum of node_count across all workers that reported back.
    total_nodes: u64,

    /// Number of workers that returned.
    workers_reported: usize
}

fn run_single(input: &ExactCoverInput, args: &Args) -> Result<RunOutcome, Box<dyn Error>> {
    let options = SolveOptions {
        timeout_ms: args.timeout.map(|s| s * 1000),
        seed: args.seed,
        luby_base: args.luby_base
    };

    let result = solve_exact_cover(input, options, None)?;
    let elapsed_ms = result.stats.common.elapsed_ms;
    let total_nodes = result.stats.common.node_count;
    let winner = Some(WorkerStats { worker_idx: 0, result });

    Ok(RunOutcome { elapsed_ms, winner, total_nodes, workers_reported: 1 })
}

fn run_parallel(input: &ExactCoverInput, args: &Args, n_workers: usize) -> Result<RunOutcome, Box<dyn Error>> {
    let cancel_atom = AtomicI32::new(0);
    let (tx, rx) = mpsc::channel::<(usize, ExactCoverResult)>();
    let base_seed: u64 = rand::random();
    let start_time = Instant::now();

    // Scoped threads: workers can borrow &cancel_atom and &input safely.
    thread::scope(|s| {
        for worker_idx in 0..n_workers {
            let tx = tx.clone();
            let cancel_ref = &cancel_atom;
            s.spawn(move || {
                let seed = base_seed.wrapping_add((worker_idx as u64).wrapping_mul(0x9E3779B9));
                let options = SolveOptions {
                    timeout_ms: args.timeout.map(|s| s * 1000),
                    seed: Some(seed),
                    luby_base: args.luby_base,
                };

                let cancel = CancelFlag::new(cancel_ref);
                match solve_exact_cover(input, options, Some(&cancel)) {
                    Ok(r) => {
                        if r.solution.is_some() {
                            cancel_ref.store(1, Ordering::Relaxed);
                        }
                        let _ = tx.send((worker_idx, r));
                    }
                    Err(e) => {
                        eprintln!("worker {worker_idx} error: {e}")
                    }
                }
            });
        }

        drop(tx);
    });

    let elapsed_ms = start_time.elapsed().as_millis() as u64;

    // Collect all results.
    let mut results: Vec<(usize, ExactCoverResult)> = rx.iter().collect();
    results.sort_by_key(|(idx, _)| *idx);

    let total_nodes: u64 = results.iter().map(|(_, r)| r.stats.common.node_count).sum();
    let workers_reported = results.len();

    let winner = results
        .into_iter()
        .filter(|(_, r)| r.solution.is_some())
        .min_by_key(|(_, r)| r.stats.common.elapsed_ms)
        .map(|(idx, r)| WorkerStats { worker_idx: idx, result: r });

    Ok(RunOutcome { elapsed_ms, winner, total_nodes, workers_reported })

}

fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    // Load input
    let raw = fs::read_to_string(&args.input)?;
    let input: ExactCoverInput = serde_json::from_str(&raw)?;

    let n_workers = match args.workers {
        0 => auto_workers(),
        n => n
    };
    let parallel_mode = n_workers > 1;

    if !args.quiet {
        eprintln!(
            "loaded input from {} ({} target cells, {} pieces)",
            args.input.display(),
            input.target_cells.len(),
            input.common.pieces.len()
        );
        if parallel_mode {
            eprintln!("parallel mode: {n_workers} workers");
        }
    }

    let mut outcomes: Vec<RunOutcome> = Vec::with_capacity(args.runs as usize);

    for run_idx in 0..args.runs {
        let outcome = if parallel_mode {
            run_parallel(&input, args, n_workers)?
        } else {
            run_single(&input, args)?
        };

        let solved = outcome.winner.is_some();

        if !args.quiet && args.runs > 1 {
            let seed_str = match &outcome.winner {
                Some(w) => format!("{:#x}", w.result.stats.common.seed),
                None => "-".to_string()
            };

            let node_str = if parallel_mode {
                format!("total_nodes: {}", outcome.total_nodes)
            } else {
                match &outcome.winner {
                    Some(w) => format!(
                        "nodes: {}, restarts: {}",
                        w.result.stats.common.node_count,
                        w.result.stats.common.restarts
                    ),
                    None => "nodes: -".to_string()
                }
            };

            eprintln!(
                "run {}/{}: {} in {}ms (seed: {}, {})",
                run_idx + 1,
                args.runs,
                if solved { "solved" } else { "no solution" },
                outcome.elapsed_ms,
                seed_str,
                node_str
            );
        }

        outcomes.push(outcome);
    }

    let last = outcomes.last().expect("at least one run executed");

    if let Some(output_path) = &args.output {
        if let Some(winner) = &last.winner {
            let json = serde_json::to_string_pretty(&winner.result)?;
            fs::write(output_path, json)?;
            if !args.quiet {
                eprintln!("wrote result to {}", output_path.display());
            }
        }
    }

    // Print summary.
    if args.json {
        if let Some(winner) = &last.winner {
            println!("{}", serde_json::to_string(&winner.result)?);
        }
    } else {
        print_summary(&outcomes, args.runs, n_workers, parallel_mode);
    }

    Ok(())
}

/// Prints a human-readable summary of the run(s).
fn print_summary(
    outcomes: &[RunOutcome],
    runs: u32,
    n_workers: usize,
    parallel_mode: bool
) {
    let last = outcomes.last().expect("at least one run");

    println!("=== Last run ===");
    match &last.winner {
        None => {
            println!("  result      : no solution");
        }
        Some(w) => {
            let s = &w.result.stats.common;
            println!("  result      : solved");
            if parallel_mode {
                println!("  workers     : {} (winner: #{})", n_workers, w.worker_idx);
                println!("  wall-clock  : {} ms", last.elapsed_ms);
                println!("  winner seed : {:#x}", s.seed);
                println!("  winner nodes: {}", s.node_count);
                println!("  winner restarts: {}", s.restarts);
                println!("  total nodes : {}", last.total_nodes);
                println!("  reported    : {}/{}", last.workers_reported, n_workers);
            } else {
                println!("  seed        : {:#x}", s.seed);
                println!("  elapsed     : {} ms", s.elapsed_ms);
                println!("  nodes       : {}", s.node_count);
                println!("  restarts    : {}", s.restarts);
                println!("  units       : {}", s.unit_propagations);
                println!("  parity      : {}", s.parity_prunes);
                println!("  island      : {}", s.island_prunes);
                println!("  dead-cell   : {}", s.dead_cell_prunes);
                println!("  neighbor    : {}", s.neighbor_prunes);
            }
            if s.timed_out { println!("  TIMED OUT"); }
            if let Some(sol) = &w.result.solution {
                println!("  placements  : {}", sol.len());
            }
        }
    }

    if runs > 1 {
        println!();
        println!("=== Aggregate over {runs} runs ===");

        let solved_count = outcomes.iter().filter(|o| o.winner.is_some()).count();
        println!("  solved      : {solved_count}/{runs}");

        let mut elapsed: Vec<u64> = outcomes.iter().map(|o| o.elapsed_ms).collect();
        elapsed.sort_unstable();
        let min = elapsed.first().copied().unwrap_or(0);
        let max = elapsed.last().copied().unwrap_or(0);
        let median = elapsed.get(elapsed.len() / 2).copied().unwrap_or(0);
        let mean = elapsed.iter().sum::<u64>() / runs as u64;
        println!("  elapsed (ms): min={min}, median={median}, mean={mean}, max={max}");

        if parallel_mode {
            let total_nodes: u64 = outcomes.iter().map(|o| o.total_nodes).sum();
            println!("  total nodes : {total_nodes} (all workers, all runs)");
        }
    }
}