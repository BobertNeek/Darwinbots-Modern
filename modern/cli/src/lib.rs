mod dna_source;
mod native_comparison;
mod probe;

use clap::{Parser, Subcommand, ValueEnum};
use darwinbots_engine::{BackendPreference, Engine, EngineConfig, LegacyDna};
use serde_json::json;
use std::hash::{Hash, Hasher};
use std::{error::Error, ffi::OsString, fs, io::Write, path::PathBuf, time::Instant};

#[derive(Debug, Parser)]
#[command(
    name = "darwinbots",
    about = "Headless Darwinbots simulation and benchmark runner"
)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Replay a genuine DB2 simulation on the Rust CPU engine.
    NativeReplay {
        #[arg(long)]
        simulation: PathBuf,
        #[arg(long, conflicts_with = "until_cycle")]
        ticks: Option<u64>,
        #[arg(long, conflicts_with = "ticks", allow_hyphen_values = true)]
        until_cycle: Option<i32>,
        #[arg(long)]
        output_sim: PathBuf,
    },
    /// Compare live native fields, including every VM memory cell.
    NativeCompare {
        #[arg(long)]
        left: PathBuf,
        #[arg(long)]
        right: PathBuf,
        #[arg(long, default_value_t = 0.0001)]
        absolute_tolerance: f64,
        #[arg(long, default_value_t = 0.000001)]
        relative_tolerance: f64,
        #[arg(long, default_value_t = 100)]
        max_differences: usize,
    },
    /// Write controlled start simulations for original executable comparisons.
    NativeSuite {
        #[arg(long)]
        directory: PathBuf,
    },
    Run {
        #[arg(long, required = true)]
        bot: Vec<PathBuf>,
        #[arg(long, default_value_t = 1)]
        population: usize,
        #[arg(long)]
        vegetable_bot: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        vegetable_population: usize,
        #[arg(long, default_value_t = 1_000)]
        initial_energy: i32,
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
        #[arg(long, default_value_t = 32_000)]
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
        Command::NativeReplay {
            simulation,
            ticks,
            until_cycle,
            output_sim,
        } => {
            let mut engine = Engine::from_native_sim(&fs::read(&simulation)?)?;
            let start = engine
                .snapshot()
                .native_cycle
                .ok_or("native simulation has no original cycle counter")?;
            let ticks = match until_cycle {
                Some(cycle) => u64::try_from(i64::from(cycle) - i64::from(start))
                    .map_err(|_| "target cycle precedes simulation cycle")?,
                None => ticks.unwrap_or(1),
            };
            if i128::from(start) + i128::from(ticks) > i128::from(i32::MAX) {
                return Err("replay exceeds the original signed Long cycle range".into());
            }
            advance(&mut engine, ticks)?;
            engine.validate_invariants()?;
            let end = engine
                .snapshot()
                .native_cycle
                .ok_or("replay lost its original cycle counter")?;
            if simulation == output_sim {
                return Err("replay output must not replace its input simulation".into());
            }
            native_comparison::write_new(&output_sim, &engine.export_native_sim()?)?;
            serde_json::to_writer(
                &mut output,
                &json!({ "start_cycle": start,
                "end_cycle": end, "ticks": ticks, "processed_tick": engine.snapshot().tick,
                "backend": "Cpu",
                "output_sim": output_sim, "population": engine.population(),
                "rng_boundary": native_comparison::RNG_BOUNDARY }),
            )?;
        }
        Command::NativeCompare {
            left,
            right,
            absolute_tolerance,
            relative_tolerance,
            max_differences,
        } => {
            let report = native_comparison::compare(
                &fs::read(left)?,
                &fs::read(right)?,
                absolute_tolerance,
                relative_tolerance,
                max_differences,
            )?;
            serde_json::to_writer(&mut output, &report)?;
            if report["passed"] != true {
                writeln!(output)?;
                return Err("native state comparison failed; see JSON differences".into());
            }
        }
        Command::NativeSuite { directory } => {
            serde_json::to_writer(&mut output, &native_comparison::write_suite(&directory)?)?;
        }
        Command::Run {
            bot,
            population,
            vegetable_bot,
            vegetable_population,
            initial_energy,
            ticks,
            backend,
            seed,
        } => {
            let report = probe::execute(probe::Options {
                bot,
                population,
                vegetable_bot,
                vegetable_population,
                initial_energy,
                ticks,
                seed,
                backend: backend.into(),
            })?;
            serde_json::to_writer(&mut output, &report)?;
        }
        Command::Bench {
            bot,
            population,
            ticks,
            warmup_ticks,
            backend,
            seed,
        } => {
            let mut engine = create_engine(population, backend, seed)?;
            let (dna, source) = read_bot(&engine, &bot)?;
            populate(&mut engine, &dna, &source, population)?;
            advance(&mut engine, warmup_ticks)?;
            let started = Instant::now();
            advance(&mut engine, ticks)?;
            let elapsed = started.elapsed().as_secs_f64().max(f64::EPSILON);
            serde_json::to_writer(
                &mut output,
                &json!({
                    "population": engine.population(),
                    "ticks": ticks,
                    "warmup_ticks": warmup_ticks,
                    "backend": format!("{:?}", engine.backend()),
                    "layout": "uniform-grid",
                    "elapsed_seconds": elapsed,
                    "ticks_per_second": ticks as f64 / elapsed,
                    "phase_ms": engine.last_phase_timings(),
                }),
            )?;
        }
        Command::Stress {
            bot,
            population,
            ticks,
            invariant_interval,
            backend,
            seed,
            metabolism_cost,
        } => {
            if invariant_interval == 0 {
                return Err("invariant interval must be positive".into());
            }
            let mut engine =
                create_engine_with_metabolism(population, backend, seed, metabolism_cost)?;
            let (dna, source) = read_bot(&engine, &bot)?;
            populate(&mut engine, &dna, &source, population)?;
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
            serde_json::to_writer(
                &mut output,
                &json!({
                    "backend": format!("{:?}", engine.backend()),
                    "ticks": ticks,
                    "population": engine.population(),
                    "invariant_checks": checks,
                    "elapsed_seconds": elapsed,
                    "final_state_hash": format!("{:016x}", hasher.finish()),
                }),
            )?;
        }
    }
    writeln!(output)?;
    Ok(())
}

fn create_engine(
    capacity: usize,
    backend: BackendArgument,
    seed: u64,
) -> Result<Engine, Box<dyn Error>> {
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
    Ok(Engine::new_native(config)?)
}

fn read_bot(
    engine: &Engine,
    path: &std::path::Path,
) -> Result<(LegacyDna, String), Box<dyn Error>> {
    dna_source::read(engine, path)
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

fn populate(
    engine: &mut Engine,
    dna: &LegacyDna,
    source: &str,
    population: usize,
) -> Result<(), Box<dyn Error>> {
    let columns = (population as f64).sqrt().ceil().max(1.0) as usize;
    let rows = population.div_ceil(columns).max(1);
    let organisms = (0..population).map(|index| {
        let column = index % columns;
        let row = index / columns;
        [
            16_000.0 * (column + 1) as f32 / (columns + 1) as f32,
            12_000.0 * (row + 1) as f32 / (rows + 1) as f32,
        ]
    });
    engine.spawn_species_batch(
        dna.clone(),
        darwinbots_engine::SpeciesId::default(),
        organisms,
        1000,
    )?;
    engine
        .retain_species_template_source(darwinbots_engine::SpeciesId::default(), source.into())?;
    Ok(())
}
