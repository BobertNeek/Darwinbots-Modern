use clap::{Parser, Subcommand, ValueEnum};
use darwinbots_engine::{BackendPreference, Engine, EngineConfig, LegacyDna};
use serde_json::json;
use std::{error::Error, ffi::OsString, fs, io::Write, path::PathBuf, time::Instant};
use std::hash::{Hash, Hasher};

#[derive(Debug, Parser)]
#[command(name = "darwinbots", about = "Headless Darwinbots simulation and benchmark runner")]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run {
        #[arg(long, required = true)]
        bot: Vec<PathBuf>,
        #[arg(long, default_value_t = 1_000)]
        ticks: u64,
        #[arg(long, value_enum, default_value_t = BackendArgument::Auto)]
        backend: BackendArgument,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    Bench {
        #[arg(long)]
        bot: PathBuf,
        #[arg(long, default_value_t = 100_000)]
        population: usize,
        #[arg(long, default_value_t = 300)]
        ticks: u64,
        #[arg(long, default_value_t = 3)]
        warmup_ticks: u64,
        #[arg(long, value_enum, default_value_t = BackendArgument::Auto)]
        backend: BackendArgument,
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
    Stress {
        #[arg(long)]
        bot: PathBuf,
        #[arg(long, default_value_t = 1)]
        population: usize,
        #[arg(long, default_value_t = 1_000_000)]
        ticks: u64,
        #[arg(long, default_value_t = 1)]
        invariant_interval: u64,
        #[arg(long, value_enum, default_value_t = BackendArgument::Cpu)]
        backend: BackendArgument,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long, default_value_t = 0)]
        metabolism_cost: i32,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum BackendArgument {
    Auto,
    Cpu,
    Gpu,
}

impl From<BackendArgument> for BackendPreference {
    fn from(value: BackendArgument) -> Self {
        match value {
            BackendArgument::Auto => Self::Auto,
            BackendArgument::Cpu => Self::Cpu,
            BackendArgument::Gpu => Self::Gpu,
        }
    }
}

pub fn run_from<I, T, W>(arguments: I, mut output: W) -> Result<(), Box<dyn Error>>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
    W: Write,
{
    match Arguments::try_parse_from(arguments)?.command {
        Command::Run { bot, ticks, backend, seed } => {
            let mut engine = create_engine(bot.len().max(1), backend, seed)?;
            for path in bot {
                engine.spawn(read_bot(&path)?)?;
            }
            let started = Instant::now();
            advance(&mut engine, ticks)?;
            let elapsed = started.elapsed().as_secs_f64();
            serde_json::to_writer(&mut output, &json!({
                "tick": engine.snapshot().tick,
                "population": engine.population(),
                "backend": format!("{:?}", engine.backend()),
                "elapsed_seconds": elapsed,
            }))?;
        }
        Command::Bench { bot, population, ticks, warmup_ticks, backend, seed } => {
            let dna = read_bot(&bot)?;
            let mut engine = create_engine(population, backend, seed)?;
            populate(&mut engine, &dna, population)?;
            advance(&mut engine, warmup_ticks)?;
            let started = Instant::now();
            advance(&mut engine, ticks)?;
            let elapsed = started.elapsed().as_secs_f64().max(f64::EPSILON);
            serde_json::to_writer(&mut output, &json!({
                "population": engine.population(),
                "ticks": ticks,
                "warmup_ticks": warmup_ticks,
                "backend": format!("{:?}", engine.backend()),
                "layout": "uniform-grid",
                "elapsed_seconds": elapsed,
                "ticks_per_second": ticks as f64 / elapsed,
                "phase_ms": engine.last_phase_timings(),
            }))?;
        }
        Command::Stress { bot, population, ticks, invariant_interval, backend, seed, metabolism_cost } => {
            if invariant_interval == 0 { return Err("invariant interval must be positive".into()); }
            let dna = read_bot(&bot)?;
            let mut engine = create_engine_with_metabolism(population, backend, seed, metabolism_cost)?;
            populate(&mut engine, &dna, population)?;
            let started = Instant::now();
            let mut checks = 0u64;
            for tick in 1..=ticks {
                engine.tick()?;
                if tick % invariant_interval == 0 || tick == ticks {
                    engine.validate_invariants()?;
                    checks += 1;
                }
            }
            let elapsed = started.elapsed().as_secs_f64();
            let mut canonical_snapshot = engine.snapshot().clone();
            canonical_snapshot.phase_timings = Default::default();
            let snapshot = serde_json::to_vec(&canonical_snapshot)?;
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            snapshot.hash(&mut hasher);
            serde_json::to_writer(&mut output, &json!({
                "backend": format!("{:?}", engine.backend()),
                "ticks": ticks,
                "population": engine.population(),
                "invariant_checks": checks,
                "elapsed_seconds": elapsed,
                "final_state_hash": format!("{:016x}", hasher.finish()),
            }))?;
        }
    }
    writeln!(output)?;
    Ok(())
}

fn create_engine(capacity: usize, backend: BackendArgument, seed: u64) -> Result<Engine, Box<dyn Error>> {
    create_engine_with_metabolism(capacity, backend, seed, 1)
}

fn create_engine_with_metabolism(
    capacity: usize,
    backend: BackendArgument,
    seed: u64,
    metabolism_cost: i32,
) -> Result<Engine, Box<dyn Error>> {
    let config = EngineConfig {
        seed,
        organism_capacity: capacity,
        backend: backend.into(),
        allow_cpu_fallback: true,
        metabolism_cost: metabolism_cost.max(0),
        ..EngineConfig::default()
    };
    Ok(Engine::new(config)?)
}

fn read_bot(path: &PathBuf) -> Result<LegacyDna, Box<dyn Error>> {
    Ok(LegacyDna::parse(&fs::read_to_string(path)?)?)
}

fn advance(engine: &mut Engine, ticks: u64) -> Result<(), Box<dyn Error>> {
    let mut remaining = ticks;
    while remaining > 0 {
        let batch = remaining.min(u32::MAX as u64) as u32;
        engine.tick_many(batch)?;
        remaining -= batch as u64;
    }
    Ok(())
}

fn populate(engine: &mut Engine, dna: &LegacyDna, population: usize) -> Result<(), Box<dyn Error>> {
    let columns = (population as f64).sqrt().ceil().max(1.0) as usize;
    let rows = population.div_ceil(columns).max(1);
    let organisms = (0..population).map(|index| {
        let column = index % columns;
        let row = index / columns;
        let position = [
            16_000.0 * (column + 1) as f32 / (columns + 1) as f32,
            12_000.0 * (row + 1) as f32 / (rows + 1) as f32,
        ];
        (dna.clone(), position)
    });
    engine.spawn_batch(organisms)?;
    Ok(())
}
