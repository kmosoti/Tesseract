use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, process::Command};
use tesseract_contracts::{Error, Result, digest};
use tesseract_journal::Session;
use tesseract_kernel::{CognitiveKernel, Kind, ValidationPolicy};
use tesseract_lab::{Protocol, SeedResult, compare, run_variant, write_exclusive};

#[derive(Parser)]
#[command(
    name = "tesseract",
    version,
    about = "A developmental symbolic learning laboratory"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Compare {
        #[arg(long, default_value = "smoke")]
        tier: Tier,
        #[arg(long, default_value = "experiments/workshop-v1.json")]
        protocol: PathBuf,
        #[arg(long, default_value_t = 1)]
        workers: usize,
        #[arg(long)]
        out: PathBuf,
    },
    Demo {
        #[arg(long)]
        journal: PathBuf,
        #[arg(long, default_value = "bayesian")]
        kind: KernelKind,
    },
    Replay {
        journal: PathBuf,
        #[arg(long)]
        checkpoint: Option<PathBuf>,
    },
    #[command(hide = true)]
    Worker {
        #[arg(long)]
        seed: u64,
        #[arg(long)]
        kind: KernelKind,
        #[arg(long)]
        protocol: PathBuf,
    },
}
#[derive(Clone, Copy, ValueEnum)]
enum Tier {
    Smoke,
    Development,
    Benchmark,
}
impl Tier {
    fn seeds(self) -> Vec<u64> {
        match self {
            Self::Smoke => (0..10).collect(),
            Self::Development => (100..120).collect(),
            Self::Benchmark => (1000..1100).collect(),
        }
    }
    fn name(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Development => "development",
            Self::Benchmark => "benchmark",
        }
    }
}
#[derive(Clone, Copy, ValueEnum)]
enum KernelKind {
    Bayesian,
    Real,
    Complex,
}
impl From<KernelKind> for Kind {
    fn from(k: KernelKind) -> Self {
        match k {
            KernelKind::Bayesian => Self::Bayesian,
            KernelKind::Real => Self::Real,
            KernelKind::Complex => Self::Complex,
        }
    }
}
fn kind_name(k: Kind) -> &'static str {
    match k {
        Kind::Bayesian => "bayesian",
        Kind::Real => "real",
        Kind::Complex => "complex",
    }
}

#[derive(Serialize)]
struct Receipt {
    schema: u32,
    tier: String,
    protocol: Protocol,
    protocol_hash: String,
    source_revision: String,
    source_dirty: bool,
    executable_sha256: String,
    machine: serde_json::Value,
    toolchain: String,
    workers: usize,
    seeds: Vec<u64>,
    results: Vec<SeedResult>,
    comparisons: Vec<tesseract_lab::Comparison>,
    limitations: Vec<String>,
}

fn run() -> Result<()> {
    // A process memory ceiling also applies to every spawned worker.
    #[cfg(target_os = "linux")]
    rlimit::setrlimit(rlimit::Resource::AS, 1024 * 1024 * 1024, 1024 * 1024 * 1024)?;
    let cli = Cli::parse();
    match cli.command {
        Commands::Worker {
            seed,
            kind,
            protocol,
        } => {
            let result = run_variant(seed, kind.into(), &Protocol::load(protocol)?)?;
            println!("{}", serde_json::to_string(&result)?);
        }
        Commands::Compare {
            tier,
            protocol,
            workers,
            out,
        } => {
            if !(1..=4).contains(&workers) {
                return Err(Error::Budget("workers must be 1..=4".into()));
            }
            if out.exists() {
                return Err(Error::Invalid("evidence output already exists".into()));
            }
            let p = Protocol::load(&protocol)?;
            let source_revision = command_text("git", &["rev-parse", "HEAD"])?;
            let source_status = command_text("git", &["status", "--porcelain"])?;
            let source_dirty = !source_status.is_empty();
            if matches!(tier, Tier::Benchmark) && source_dirty {
                return Err(Error::Rejected(
                    "benchmark requires a clean frozen checkout".into(),
                ));
            }
            let seeds = tier.seeds();
            let executable = std::env::current_exe()?;
            let executable_sha256 = format!("{:x}", Sha256::digest(std::fs::read(&executable)?));
            let jobs: Vec<_> = seeds
                .iter()
                .flat_map(|seed| Kind::ALL.map(|kind| (*seed, kind)))
                .collect();
            let mut results = Vec::new();
            for chunk in jobs.chunks(workers) {
                let batch = std::thread::scope(|scope| {
                    let mut handles = Vec::new();
                    for (seed, kind) in chunk {
                        let exe = &executable;
                        let protocol = &protocol;
                        handles.push(scope.spawn(move || -> Result<SeedResult> {
                            let output = Command::new(exe)
                                .arg("worker")
                                .args([
                                    "--seed",
                                    &seed.to_string(),
                                    "--kind",
                                    kind_name(*kind),
                                    "--protocol",
                                ])
                                .arg(protocol)
                                .output()?;
                            if !output.status.success() {
                                return Err(Error::Rejected(format!(
                                    "worker {seed}/{kind:?} failed: {}",
                                    String::from_utf8_lossy(&output.stderr)
                                )));
                            }
                            Ok(serde_json::from_slice(&output.stdout)?)
                        }));
                    }
                    handles
                        .into_iter()
                        .map(|h| {
                            h.join()
                                .map_err(|_| Error::Rejected("worker thread panicked".into()))?
                        })
                        .collect::<Result<Vec<_>>>()
                })?;
                results.extend(batch);
                eprintln!(
                    "completed {}/{} seed/representation runs",
                    results.len(),
                    jobs.len()
                );
            }
            let comparisons = compare(&results, &p, matches!(tier, Tier::Benchmark))?;
            if source_revision != command_text("git", &["rev-parse", "HEAD"])?
                || source_status != command_text("git", &["status", "--porcelain"])?
                || executable_sha256 != format!("{:x}", Sha256::digest(std::fs::read(&executable)?))
            {
                return Err(Error::Rejected(
                    "source or executable changed during comparison".into(),
                ));
            }
            let toolchain = command_text("rustc", &["--version"])?;
            let receipt = Receipt {
                schema: 1, tier: tier.name().into(), protocol_hash: digest(&p)?, protocol: p,
                source_revision, source_dirty, executable_sha256, toolchain, workers, seeds, results, comparisons,
                machine: serde_json::json!({"os": std::env::consts::OS, "architecture": std::env::consts::ARCH,
                    "logical_cpus": std::thread::available_parallelism().map(|v| v.get()).ok(),
                    "cpu_model": std::fs::read_to_string("/proc/cpuinfo").ok().and_then(|s| s.lines().find_map(|l| l.strip_prefix("model name").and_then(|v| v.split_once(':').map(|(_, v)| v.trim().to_owned()))))}),
                limitations: vec![
                    "Finite four-state material worlds; no general-intelligence or physical-quantum claim.".into(),
                    "Density candidates are seeded from empirical classical transitions; this is a constrained representation experiment, not a new learning-from-scratch algorithm.".into(),
                    "Stored coordinate counts differ; positive substrate claims require capacity-matched replication and additional graph/rule and VSA baselines.".into(),
                    "Linux peak RSS is worker-process high-water memory including runtime and all controls; timing is observational and excluded from behavioral hashes.".into(),
                    "Ontology expansion and open-ended primitive invention are subsequent milestones.".into(),
                ],
            };
            write_exclusive(&out, &receipt)?;
            println!("{}", serde_json::to_string_pretty(&receipt.comparisons)?);
        }
        Commands::Demo { journal, kind } => {
            let data = tesseract_workshop::dataset(100, 0.0)?;
            let mut session = Session::create(&journal, kind.into())?;
            for e in data.training {
                session.observe(e)?;
            }
            let candidate = session
                .kernel()
                .develop(tesseract_contracts::Budget::default(), 100)?;
            let proof = session.kernel().validate(
                candidate,
                &data.validation,
                ValidationPolicy::default(),
            )?;
            let report = proof.report().clone();
            session.commit(proof)?;
            let snapshot = journal.with_extension("checkpoint.json");
            session.checkpoint(&snapshot)?;
            let expected = session.kernel().state_digest()?;
            drop(session);
            let restored = Session::open(journal)?;
            restored.verify_checkpoint(snapshot)?;
            println!(
                "{}",
                serde_json::json!({"revision":restored.kernel().revision(),"processes":restored.kernel().processes().len(),"replay_matches":expected==restored.kernel().state_digest()?,"validation":report})
            );
        }
        Commands::Replay {
            journal,
            checkpoint,
        } => {
            let restored = Session::open(journal)?;
            if let Some(p) = checkpoint {
                restored.verify_checkpoint(p)?;
            }
            println!(
                "{}",
                serde_json::json!({"revision":restored.kernel().revision(),"state_hash":restored.kernel().state_digest()?,"journal_head":restored.head(),"processes":restored.kernel().processes().len()})
            );
        }
    }
    Ok(())
}
fn command_text(program: &str, args: &[&str]) -> Result<String> {
    let out = Command::new(program).args(args).output()?;
    if !out.status.success() {
        return Err(Error::Invalid(format!(
            "{program} provenance command failed"
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
