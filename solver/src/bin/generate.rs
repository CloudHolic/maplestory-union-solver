// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Generator CLI for ML training instances.
//!
//! Synthesizes random `ExactCoverInput`s and runs the solver with branch tracing.
//! Successful solves emit JSONL records to a single output file.

#[cfg(not(target_arch = "wasm32"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::error::Error;
use std::fs::{File, create_dir_all};
use std::io::{BufWriter, Write};
use std::mem::take;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex, mpsc};
use std::sync::atomic::{AtomicI32, Ordering};
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
    /// Output JSONL file path.
    #[arg(short, long)]
    out: PathBuf,

    /// Maximum uncompressed size per shard, in megabytes.
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

/// Stats from one instance's parallel race.
struct WorkerStats {
    worker_idx: usize,
    node_count: u64,
    trace_bytes: Vec<u8>
}

/// Outcome of one instance's parallel race.
struct RunOutcome {
    /// Elapsed time for the race.
    elapsed_ms: u64,

    /// The winning worker's stats and trace bytes.
    winner: Option<WorkerStats>,

    /// Number of workers that returned.
    workers_reported: usize
}

/// `Write` adapter funneling into a thread-shared byte buffer.
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Rotating, gzip-compressed JSONL shard writer.
struct ShardWriter {
    out_dir: PathBuf,
    max_uncompressed_bytes: u64,
    next_shard_idx: u32,
    current: Option<CurrentShard>
}

struct CurrentShard {
    encoder: GzEncoder<BufWriter<File>>,
    uncompressed_written: u64,
    path: PathBuf
}

impl ShardWriter {
    fn new(out_dir: PathBuf, max_uncompressed_bytes: u64) -> std::io::Result<Self> {
        create_dir_all(&out_dir)?;
        Ok(Self {
            out_dir,
            max_uncompressed_bytes,
            next_shard_idx: 0,
            current: None
        })
    }

    fn write_instance(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        if let Some(c) = &self.current {
            if c.uncompressed_written + bytes.len() as u64 > self.max_uncompressed_bytes {
                self.close_current()?;
            }
        }

        if self.current.is_none() {
            self.open_new_shard()?;
        }

        let c = self.current.as_mut().expect("current shard just opened");
        c.encoder.write_all(bytes)?;
        c.uncompressed_written += bytes.len() as u64;
        Ok(())
    }

    fn open_new_shard(&mut self) -> std::io::Result<()> {
        let path = self.out_dir
            .join(format!("synth-{:04}.jsonl.gz", self.next_shard_idx));
        let file = File::create(&path)?;
        let buf = BufWriter::new(file);
        let encoder = GzEncoder::new(buf, Compression::default());

        self.current = Some(CurrentShard {
            encoder,
            uncompressed_written: 0,
            path
        });

        self.next_shard_idx += 1;
        Ok(())
    }

    fn close_current(&mut self) -> std::io::Result<()> {
        if let Some(c) = self.current.take() {
            let buf = c.encoder.finish()?;
            buf.into_inner()
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
    base_seed: u64
) -> Result<RunOutcome, Box<dyn Error>> {
    let cancel_atom = AtomicI32::new(0);
    let buffers: Vec<Arc<Mutex<Vec<u8>>>> = (0..n_workers)
        .map(|_| Arc::new(Mutex::new(Vec::new())))
        .collect();
    let (tx, rx) = mpsc::channel::<(usize, u64)>();
    let start_time = Instant::now();

    thread::scope(|s| {
        for (worker_idx, buf) in buffers.iter().enumerate() {
            let buf = Arc::clone(buf);
            let tx = tx.clone();
            let cancel_ref = &cancel_atom;
            let instance_id = instance_id.to_string();

            s.spawn(move || {
                let writer = SharedBuf(buf);
                let mut tracer = Tracer::new(Box::new(writer));
                tracer.set_instance_id(instance_id);

                let seed = base_seed.wrapping_add((worker_idx as u64).wrapping_mul(RATIO));
                let options = SolveOptions {
                    timeout_ms: args.timeout.map(|s| s * 1000),
                    seed: Some(seed),
                    luby_base: args.luby_base
                };

                let cancel = CancelFlag::new(cancel_ref);
                match solve_exact_cover(input, options, Some(&cancel), Some(&mut tracer)) {
                    Ok(r) => {
                        let success = r.solution.is_some();
                        if success {
                            cancel_ref.store(1, Ordering::Relaxed);
                        }

                        let _ = tx.send((
                            worker_idx,
                            if success {
                                r.stats.common.node_count
                            } else {
                                0
                            }
                        ));
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
    let mut results: Vec<(usize, u64)> = rx.iter().collect();
    let workers_reported = results.len();
    results.sort_by_key(|(idx, _)| *idx);

    // Winner = lowest-elapsed_ms worker that produced a non-zero node_count.
    let winner = results
        .into_iter()
        .filter(|(_, nodes)| *nodes > 0)
        .next()
        .map(|(idx, nodes)| {
            let bytes = take(&mut *buffers[idx].lock().unwrap());
            WorkerStats {
                worker_idx: idx,
                node_count: nodes,
                trace_bytes: bytes
            }
        });

    Ok(RunOutcome { elapsed_ms, winner, workers_reported })
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

    let mut shard_writer = ShardWriter::new(args.out.clone(), args.shard_size_mb * 1024 * 1024)?;
    let mut master_rng = Xoshiro256PlusPlus::seed_from_u64(master_seed);
    let mut succeeded = 0u32;
    let mut failed = 0u32;

    for i in 0..args.count {
        let input = build_instance(&catalog, &board, &mut master_rng);
        let instance_id = format!("synth_{i:08}");
        let base_seed = master_rng.next_u64();

        let outcome = run_parallel(&input, &instance_id, args, n_workers, base_seed)?;

        match &outcome.winner {
            Some(w) => {
                shard_writer.write_instance(&w.trace_bytes)?;
                succeeded += 1;
                if !args.quiet {
                    eprintln!(
                        "[{:>4}/{:<4}] {} solved in {}ms (worker #{}, {} nodes, {} bytes)",
                        i + 1, args.count, instance_id,
                        outcome.elapsed_ms, w.worker_idx, w.node_count, w.trace_bytes.len()
                    );
                }
            }

            None => {
                failed += 1;
                if !args.quiet {
                    eprintln!(
                        "[{:>4}/{:<4}] {} all {} workers failed in {}ms",
                        i + 1, args.count, instance_id,
                        outcome.workers_reported, outcome.elapsed_ms
                    );
                }
            }
        }
    }

    shard_writer.finish()?;
    eprintln!("\ndone. {} succeeded, {} failed.", succeeded, failed);
    Ok(())
}