// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Generator CLI for ML training instances.
//!
//! Synthesizes random `ExactCoverInput`s and runs the solver with branch tracing.
//! Successful solves emit JSONL records to a single output file.

use std::error::Error;
use std::fs::{File, create_dir_all, metadata, remove_dir_all, remove_file, create_dir};
use std::io::{BufWriter, Write, copy};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

use clap::Parser;
use flate2::Compression;
use flate2::write::GzEncoder;
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256PlusPlus;

use solver::{
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
    /// Output directory for shards. Per-worker temp files go under `<out>/tmp/`.
    #[arg(short, long)]
    out: PathBuf,

    /// Maximum compressed size per shard, in megabytes.
    #[arg(long, default_value_t = 100)]
    shard_size_mb: u64,

    /// Number of instances to synthesize.
    #[arg(short, long, default_value_t = 100)]
    count: usize,

    /// Master seed for instance synthesis and worker seed seeding.
    /// If omitted, draws from system entropy.
    #[arg(short, long)]
    seed: Option<u64>,

    /// Per-instance solver timeout in seconds. No timeout if omitted.
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

    /// Suppress per-instance progress output.
    #[arg(short, long)]
    quiet: bool
}


//noinspection DuplicatedCode
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

/// One worker's result within a parallel race.
///
/// `temp_path` is always set - even on failure or solver error - so the
/// main thread can unconditionally unlink the file after the race ends.
struct WorkerResult {
    worker_idx: usize,
    succeeded: bool,
    node_count: u64,
    temp_path: PathBuf
}

/// Outcome of one instance's parallel race.
struct RunOutcome {
    /// Elapsed time for the race.
    elapsed_ms: u64,

    /// First successful worker, if any.
    winner: Option<WorkerResult>,

    /// All other workers (failed or solver-errored).
    /// Their temp files exist on disk and must be unlinked.
    losers: Vec<WorkerResult>,

    /// Total workers that reported back (= winner.is_some() as usize + losers.len())
    workers_reported: usize
}

/// Rotating shard writer.
///
/// appends per-instance gzip-compressed traces to the current shard by raw byte copy.
struct ShardWriter {
    out_dir: PathBuf,
    /// Cap on the compressed size of each shard.
    max_bytes: u64,
    next_shard_idx: u32,
    current: Option<CurrentShard>
}

struct CurrentShard {
    writer: BufWriter<File>,
    bytes_written: u64,
    path: PathBuf
}

impl ShardWriter {
    fn new(out_dir: PathBuf, max_bytes: u64) -> std::io::Result<Self> {
        create_dir_all(&out_dir)?;
        Ok(Self {
            out_dir,
            max_bytes,
            next_shard_idx: 0,
            current: None
        })
    }

    /// Appends the entire contents of `src` to the current shard.
    /// Rotates first if needed.
    fn append_file(&mut self, src: &Path) -> std::io::Result<()> {
        let src_size = metadata(src)?.len();

        if let Some(c) = &self.current
            && c.bytes_written + src_size > self.max_bytes {
            self.close_current()?;
        }

        if self.current.is_none() {
            self.open_new_shard()?;
        }

        let c = self.current.as_mut().expect("current shard just opened");
        let mut src_file = File::open(src)?;
        let copied = copy(&mut src_file, &mut c.writer)?;
        c.bytes_written += copied;
        Ok(())
    }

    fn open_new_shard(&mut self) -> std::io::Result<()> {
        let path = self.out_dir
            .join(format!("synth-{:04}.jsonl.gz", self.next_shard_idx));
        let file = File::create(&path)?;
        let writer = BufWriter::new(file);

        self.current = Some(CurrentShard { writer, bytes_written: 0, path });
        self.next_shard_idx += 1;
        Ok(())
    }

    fn close_current(&mut self) -> std::io::Result<()> {
        if let Some(mut c) = self.current.take() {
            c.writer.flush()?;
            c.writer.into_inner()
                .map_err(|e| std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("flushing shard {:?}: {}", c.path, e.error())
                ))?
                .sync_all()?;
        }

        Ok(())
    }

    fn finish(mut self) -> std::io::Result<()> {
        self.close_current()
    }
}

fn run_parallel(
    input: &ExactCoverInput,
    instance_id: &str,
    args: &Args,
    n_workers: usize,
    base_seed: u64,
    temp_dir: &Path
) -> Result<RunOutcome, Box<dyn Error>> {
    let cancel_atom = AtomicI32::new(0);
    let (tx, rx) = mpsc::channel::<WorkerResult>();
    let start_time = Instant::now();

    // Pre-compute every worker's temp file path so the main thread can clean them up
    // even if a worker panics before sending its result.
    let temp_paths: Vec<PathBuf> = (0..n_workers)
        .map(|idx| temp_dir.join(format!("w{idx}_{instance_id}.jsonl.gz")))
        .collect();

    thread::scope(|s| {
        for (worker_idx, temp_path) in temp_paths.iter().enumerate() {
            let tx = tx.clone();
            let cancel_ref = &cancel_atom;
            let instance_id = instance_id.to_string();
            let temp_path = temp_path.clone();

            s.spawn(move || {
                let result = run_worker(
                    input, &*instance_id, args, worker_idx,
                    base_seed, cancel_ref, &temp_path
                );
                let _ = tx.send(result);
            });
        }

        drop(tx);
    });

    let elapsed_ms = start_time.elapsed().as_millis() as u64;

    let mut results: Vec<WorkerResult> = rx.iter().collect();
    results.sort_by_key(|r| r.worker_idx);
    let workers_reported = results.len();

    // First success is the winner; everything else is a loser to clean up.
    let mut winner = None;
    let mut losers = Vec::new();
    for r in results {
        if winner.is_none() && r.succeeded {
            winner = Some(r);
        } else {
            losers.push(r);
        }
    }

    Ok(RunOutcome { elapsed_ms, winner, losers, workers_reported })
}

/// Runs a single worker.
fn run_worker(
    input: &ExactCoverInput,
    instance_id: &str,
    args: &Args,
    worker_idx: usize,
    base_seed: u64,
    cancel_ref: &AtomicI32,
    temp_path: &Path
) -> WorkerResult {
    let file = match File::create(temp_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("worker {worker_idx} failed to create temp file {temp_path:?}: {e}");
            return WorkerResult {
                worker_idx, succeeded: false, node_count: 0,
                temp_path: temp_path.to_path_buf()
            };
        }
    };

    let encoder = GzEncoder::new(BufWriter::new(file), Compression::default());

    let mut tracer = Tracer::new(Box::new(encoder));
    tracer.set_instance_id(instance_id.to_string());

    let seed = base_seed.wrapping_add((worker_idx as u64).wrapping_mul(RATIO));
    let options = SolveOptions {
        timeout_ms: args.timeout.map(|s| s * 1000),
        seed: Some(seed),
        luby_base: args.luby_base
    };

    let cancel = CancelFlag::new(cancel_ref);
    let solve_result = solve_exact_cover(input, options, Some(&cancel), Some(&mut tracer));

    drop(tracer);

    match solve_result {
        Ok(r) => {
            let succeeded = r.solution.is_some();
            if succeeded {
                cancel_ref.store(1, Ordering::Relaxed);
            }

            WorkerResult {
                worker_idx, succeeded,
                node_count: if succeeded { r.stats.common.node_count } else { 0 },
                temp_path: temp_path.to_path_buf()
            }
        }

        Err(e) => {
            eprintln!("worker {worker_idx} error: {e}");
            WorkerResult {
                worker_idx, succeeded: false, node_count: 0,
                temp_path: temp_path.to_path_buf()
            }
        }
    }
}

fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    let board = UnionBoard::new();
    let catalog = PolyominoCatalog::enumerate(6);
    let n_workers = match args.workers {
        0 => auto_workers(),
        n => n
    };

    let master_seed = args.seed.unwrap_or_else(|| rand::random::<u64>());
    if !args.quiet {
        eprintln!(
            "generating {} instances ({} workers each, seed {:#x})",
            args.count, n_workers, master_seed
        );
    }

    // Wipe any leftover temp files from a previous interrupted run
    // so disk usage doesn't accumulate silently across batches.
    let temp_dir = args.out.join("tmp");
    if temp_dir.exists() {
        remove_dir_all(&temp_dir)?;
    }
    create_dir(&temp_dir)?;

    let mut shard_writer = ShardWriter::new(args.out.clone(), args.shard_size_mb * 1024 * 1024)?;
    let mut master_rng = Xoshiro256PlusPlus::seed_from_u64(master_seed);
    let mut succeeded = 0u32;
    let mut failed = 0u32;

    for i in 0..args.count {
        let input = build_instance(&catalog, &board, &mut master_rng);
        let instance_id = format!("synth_{i:08}");
        let base_seed = master_rng.next_u64();

        let outcome = run_parallel(&input, &instance_id, args, n_workers, base_seed, &temp_dir)?;

        if let Some(w) = &outcome.winner {
            let bytes_written = metadata(&w.temp_path)?.len();
            shard_writer.append_file(&w.temp_path)?;
            succeeded += 1;

            if !args.quiet {
                eprintln!(
                    "[{:>4}/{:<4}] {} solved in {}ms (worker #{}, {} nodes, {} compressed bytes)",
                    i + 1, args.count, instance_id, outcome.elapsed_ms,
                    w.worker_idx, w.node_count, bytes_written
                );
            }
        } else {
            failed += 1;
            if !args.quiet {
                eprintln!(
                    "[{:>4}/{:<4}] {} all {} workers failed in {}ms",
                    i + 1, args.count, instance_id,
                    outcome.workers_reported, outcome.elapsed_ms
                );
            }
        }

        // Unlink all temp files (winner already appended, losers discarded).
        if let Some(w) = &outcome.winner {
            let _ = remove_file(&w.temp_path);
        }
        for l in &outcome.losers {
            let _ = remove_file(&l.temp_path);
        }
    }

    shard_writer.finish()?;
    let _ = remove_dir_all(&temp_dir);
    eprintln!("\ndone. {} succeeded, {} failed.", succeeded, failed);
    Ok(())
}