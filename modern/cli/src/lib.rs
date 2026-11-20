mod dna_source;
mod native_comparison;
mod probe;

use clap::{Parser, Subcommand, ValueEnum};
use darwinbots_engine::{BackendPreference, Engine, EngineConfig, SpeciesDefinition, SpeciesId};
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
    /// A validated zero-tick replay preserves the original file bytes.
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
            let input_bytes = fs::read(&simulation)?;
            let mut engine = Engine::from_native_sim(&input_bytes)?;
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
            // A zero-update file replay is a validated lossless copy. Loading
            // a runtime still applies original LoadRobotBody normalization;
            // positive updates always export that actual live engine state.
            let output_bytes = if ticks == 0 {
                input_bytes
            } else {
                engine.export_native_sim()?
            };
            native_comparison::write_new(&output_sim, &output_bytes)?;
            serde_json::to_writer(
                &mut output,
                &json!({ "start_cycle": start,
                "end_cycle": end, "ticks": ticks, "processed_tick": engine.snapshot().tick,
                "backend": "Cpu",
                "output_sim": output_sim, "population": engine.population(),
                "output_image": if ticks == 0 { "unchanged_input" } else { "engine_native_export" },
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
            let bot = read_bot(&engine, &bot)?;
            populate(&mut engine, &bot, population)?;
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
            let bot = read_bot(&engine, &bot)?;
            populate(&mut engine, &bot, population)?;
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
) -> Result<dna_source::BotSource, Box<dyn Error>> {
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
    bot: &dna_source::BotSource,
    population: usize,
) -> Result<(), Box<dyn Error>> {
    let species = if let Some(profile) = &bot.mutation_profile {
        engine.register_species(SpeciesDefinition {
            name: bot.name.clone(),
            mutation_profile: Some(profile.clone()),
            ..SpeciesDefinition::default()
        })
    } else {
        SpeciesId::default()
    };
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
    engine.spawn_species_batch(bot.dna.clone(), species, organisms, 1000)?;
    engine.retain_species_template_source(species, bot.source.clone())?;
    Ok(())
}

#[cfg(test)]
mod file_profile_tests {
    use super::*;
    use darwinbots_engine::{LegacyDna, MutationRate, SaveFile, StandardMutationProfile};

    #[test]
    fn benchmark_import_profile_is_species_scoped_and_persists_without_source_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("selected.txt");
        let source = "' retained file\r\nstart 7 900 store stop\r\n";
        fs::write(&path, source).unwrap();
        let profile = StandardMutationProfile {
            point: MutationRate {
                one_in: 37.25,
                mean: 3.0,
                std_dev: 0.25,
            },
            point_value_change_percent: 13,
            ..Default::default()
        };
        fs::write(path.with_extension("mrate"), profile.to_mrate().unwrap()).unwrap();
        let mut engine = Engine::new_native(EngineConfig {
            organism_capacity: 16,
            ..EngineConfig::testing()
        })
        .unwrap();
        let unrelated = engine
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [10.0, 10.0])
            .unwrap();
        let other_profile = StandardMutationProfile {
            point_value_change_percent: 99,
            ..Default::default()
        };
        engine
            .set_mutation_profile(unrelated, other_profile.clone())
            .unwrap();
        let default_profile = engine.default_mutation_profile().clone();
        let loaded = read_bot(&engine, &path).unwrap();
        populate(&mut engine, &loaded, 2).unwrap();
        assert_eq!(engine.population(), 3);
        assert_eq!(engine.mutation_profile(unrelated).unwrap(), &other_profile);
        assert_eq!(engine.default_mutation_profile(), &default_profile);
        let imported: Vec<_> = engine
            .snapshot()
            .organisms
            .iter()
            .filter(|bot| bot.id != unrelated)
            .map(|bot| (bot.id, bot.species))
            .collect();
        assert_eq!(imported.len(), 2);
        assert_ne!(imported[0].1, SpeciesId::default());
        for &(id, species) in &imported {
            assert_eq!(species, imported[0].1);
            assert_eq!(engine.mutation_profile(id).unwrap(), &profile);
            assert_eq!(engine.export_species_template_dna(species).unwrap(), source);
        }
        let state = serde_json::to_value(&engine).unwrap();
        assert_eq!(
            state["species"][imported[0].1.0 as usize]["mutation_profile"],
            serde_json::to_value(&profile).unwrap()
        );
        assert_eq!(
            state["species"][imported[0].1.0 as usize]["mutation_rate"],
            0.0
        );
        let saved = SaveFile::encode(&engine).unwrap();
        fs::remove_file(path.with_extension("mrate")).unwrap();
        fs::remove_file(path).unwrap();
        let restored = SaveFile::decode(&saved).unwrap();
        assert_eq!(
            restored.mutation_profile(unrelated).unwrap(),
            &other_profile
        );
        for (id, _) in imported {
            assert_eq!(restored.mutation_profile(id).unwrap(), &profile);
        }
        assert_eq!(SaveFile::encode(&restored).unwrap(), saved);
    }

    #[test]
    fn missing_sidecar_keeps_existing_default_species_and_profile() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("plain.txt");
        fs::write(&path, "start stop").unwrap();
        let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
        let configured = StandardMutationProfile {
            point_value_change_percent: 91,
            ..Default::default()
        };
        engine
            .set_default_mutation_profile(configured.clone())
            .unwrap();
        let loaded = read_bot(&engine, &path).unwrap();
        assert!(loaded.mutation_profile.is_none());
        populate(&mut engine, &loaded, 2).unwrap();
        for robot in &engine.snapshot().organisms {
            assert_eq!(robot.species, SpeciesId::default());
            assert_eq!(engine.mutation_profile(robot.id).unwrap(), &configured);
        }
        assert_eq!(engine.default_mutation_profile(), &configured);
    }

    #[test]
    fn all_real_cli_file_commands_require_valid_optional_sidecars_before_import() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("file.txt");
        fs::write(&path, "start stop").unwrap();
        let sidecar = path.with_extension("mrate");
        let profile = StandardMutationProfile {
            point_value_change_percent: 13,
            ..Default::default()
        };
        let valid = profile.to_mrate().unwrap();
        for command in ["run", "bench", "stress"] {
            let mut arguments = vec![
                "darwinbots".to_owned(),
                command.into(),
                "--bot".into(),
                path.to_string_lossy().into_owned(),
                "--population".into(),
                "1".into(),
                "--ticks".into(),
                "0".into(),
                "--backend".into(),
                "cpu".into(),
            ];
            if command == "bench" {
                arguments.extend(["--warmup-ticks".into(), "0".into()]);
            }
            fs::write(&sidecar, &valid).unwrap();
            let mut output = Vec::new();
            run_from(arguments.clone(), &mut output).unwrap();
            let report: serde_json::Value = serde_json::from_slice(&output).unwrap();
            assert_eq!(report["population"], 1);
            assert_eq!(report["backend"], "Cpu");
            fs::write(&sidecar, "not a mutation profile\r\n").unwrap();
            output.clear();
            let error = run_from(arguments, &mut output).unwrap_err();
            assert!(error.to_string().contains("mrate"));
            assert!(
                output.is_empty(),
                "{command} must not publish a successful partial report"
            );
        }
        let engine = Engine::new_native(EngineConfig::testing()).unwrap();
        let saved = SaveFile::encode(&engine).unwrap();
        assert!(read_bot(&engine, &path).is_err());
        assert_eq!(SaveFile::encode(&engine).unwrap(), saved);
    }
}
