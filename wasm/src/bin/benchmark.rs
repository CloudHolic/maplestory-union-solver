// SPDX-License-Identifier: LGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Benchmark CLI for the ExactCover solver.
//!
//! Reads a JSON input file, runs the solver one or more times, and reports timing statistics.
//! Supports writing the JSON result to a file.

use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use maplestory_union_solver_wasm::{
    ExactCoverInput, ExactCoverResult, SolveOptions, solve_exact_cover
};

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

fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    // Load input
    let raw = fs::read_to_string(&args.input)?;
    let input: ExactCoverInput = serde_json::from_str(&raw)?;

    if !args.quiet {
        eprintln!(
            "loaded input from {} ({} target cells, {} pieces)",
            args.input.display(),
            input.target_cells.len(),
            input.common.pieces.len()
        );
    }

    // Run solver
    let mut last_result: Option<ExactCoverResult> = None;
    let mut all_elapsed_ms: Vec<u64> = Vec::with_capacity(args.runs as usize);
    let mut all_solved: Vec<bool> = Vec::with_capacity(args.runs as usize);

    for run_idx in 0..args.runs {
        let options = SolveOptions {
            timeout_ms: args.timeout.map(|s| s * 1000),
            seed: args.seed,
            luby_base: args.luby_base
        };

        let result = solve_exact_cover(&input, options)?;
        let solved = result.solution.is_some();
        all_elapsed_ms.push(result.stats.common.elapsed_ms);
        all_solved.push(solved);

        if !args.quiet && args.runs > 1 {
            eprintln!(
                "run {}/{}: {} in {}ms (seed: {:#x}, nodes: {}, restarts: {}",
                run_idx + 1,
                args.runs,
                if solved { "solved" } else { "no solution" },
                result.stats.common.elapsed_ms,
                result.stats.common.seed,
                result.stats.common.node_count,
                result.stats.common.restarts
            );
        }

        last_result = Some(result);
    }

    let last_result = last_result.expect("at least one run executed");

    // Write output file (if requested)
    if let Some(output_path) = &args.output {
        let json = serde_json::to_string_pretty(&last_result)?;
        fs::write(output_path, json)?;
        if !args.quiet {
            eprintln!("wrote result to {}", output_path.display());
        }
    }

    // Print summary
    if args.json {
        // JSON to stdout
        println!("{}", serde_json::to_string(&last_result)?);
    } else {
        print_summary(&last_result, &all_elapsed_ms, &all_solved, args.runs)
    }

    Ok(())
}

/// Prints a human-readable summary of the run(s).
fn print_summary(
    last: &ExactCoverResult,
    elapsed_ms: &[u64],
    solved: &[bool],
    runs: u32
) {
    let stats = &last.stats.common;

    println!("=== Last run ===");
    println!(
        "  result      : {}",
        if last.solution.is_some() { "solved" } else { "no solution" }
    );
    println!("  seed        : {:#x}", stats.seed);
    println!("  elapsed     : {} ms", stats.elapsed_ms);
    println!("  nodes       : {}", stats.node_count);
    println!("  restarts    : {}", stats.restarts);
    println!("  units       : {}", stats.unit_propagations);
    println!("  parity      : {}", stats.parity_prunes);
    println!("  island      : {}", stats.island_prunes);
    println!("  dead-cell   : {}", stats.dead_cell_prunes);
    println!("  neighbor    : {}", stats.neighbor_prunes);

    if stats.timed_out {
        println!("  TIMED OUT");
    }

    if let Some(sol) = &last.solution {
        println!("  placements  : {}", sol.len());
    }

    if runs > 1 {
        println!();
        println!("=== Aggregate over {runs} runs ===");

        let solved_count = solved.iter().filter(|&&s| s).count();
        println!("  solved      : {solved_count}/{runs}");

        let mut sorted = elapsed_ms.to_vec();
        sorted.sort_unstable();
        let min = *sorted.first().unwrap_or(&0);
        let max = *sorted.last().unwrap_or(&0);
        let median = sorted.get(sorted.len() / 2).copied().unwrap_or(0);
        let total: u64 = sorted.iter().sum();
        let mean = total / runs as u64;

        println!("  elapsed (ms): min={min}, median={median}, mean={mean}, max={max}");
    }
}