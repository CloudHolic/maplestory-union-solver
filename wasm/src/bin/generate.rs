// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Generator CLI for ML training instances.
//!
//! Synthesizes random `ExactCoverInput`s and runs the solver with branch tracing.
//! Successful solves emit JSONL records to a single output file.

use std::error::Error;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex, mpsc};
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;
use std::time::Instant;

use clap::Parser;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

use maplestory_union_solver_wasm::{
    CancelFlag, ExactCoverInput, PolyominoCatalog, SolveOptions, Tracer, UnionBoard,
    build_instance, solve_exact_cover
};

const RATIO: u64 = 0x9E3779B9;

fn auto_workers() -> usize {
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .saturating_sub(1)
        .max(2)
}

#[derive(Parser, Debug)]
#[command(version, about = "Synthesize ML training instances", long_about = None)]
struct Args {
    /// Output JSONL file path.
    #[arg(short, long)]
    out: PathBuf,

    /// Number of instances to synthesize.
    #[arg(short, long, default_value_t = 100)]
    count: usize,

    /// Master seed for instance synthesis and worker seed seeding.
    #[arg(short, long, default_value_t = 42)]
    seed: u64,

    /// Per-instance solver timeout in seconds. No timeout if omitted.
    #[arg(short, long)]
    timeout: Option<u64>,

    #[arg(short = 'b', long, )]
    luby_base: u64,

    workers: usize,

    quiet: bool
}

fn main() {

}