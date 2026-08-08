use darwinbots_engine::{
    BackendPreference, Engine, EngineConfig, LegacyDna, OrganismId, Snapshot, SpeciesDefinition,
    SpeciesId,
};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

pub(crate) struct Options {
    pub bot: Vec<PathBuf>,
    pub population: usize,
    pub vegetable_bot: Option<PathBuf>,
    pub vegetable_population: usize,
    pub initial_energy: i32,
    pub ticks: u64,
    pub seed: u64,
    pub backend: BackendPreference,
}

pub(crate) fn execute(options: Options) -> Result<Value, Box<dyn Error>> {
    if options.population == 0 {
        return Err("population must be positive".into());
    }
    if options.initial_energy <= 0 {
        return Err("initial energy must be positive".into());
    }
    if options.vegetable_population > 0 && options.vegetable_bot.is_none() {
        return Err("vegetable bot is required when vegetable population is positive".into());
    }

    let starting_population = options
        .bot
        .len()
        .checked_mul(options.population)
        .and_then(|value| value.checked_add(options.vegetable_population))
        .ok_or("starting population overflow")?;
    let candidate_starting_population = options
        .bot
        .len()
        .checked_mul(options.population)
        .ok_or("candidate starting population overflow")?;
    let positions = grid_positions(starting_population, 16_000.0, 12_000.0);
    let mut cursor = 0usize;
    let mut warnings = Vec::new();
    let mut candidate_species = Vec::with_capacity(options.bot.len());
    let mut engine = Engine::new(EngineConfig {
        seed: options.seed,
        organism_capacity: 25_000usize.max(starting_population.saturating_add(1)),
        vegetable_population_cap: 500usize.max(options.vegetable_population),
        world_width: 16_000.0,
        world_height: 12_000.0,
        backend: options.backend,
        allow_cpu_fallback: true,
        metabolism_cost: 1,
        ..EngineConfig::default()
    })?;

    for path in &options.bot {
        let dna = read_dna(path)?;
        warnings.extend(dna.compatibility_warnings().iter().cloned());
        let species = engine.register_species(SpeciesDefinition {
            name: file_name(path),
            vegetable: false,
            minimum_population: 0,
            reseed: false,
            mutation_rate: 0.0,
            ..SpeciesDefinition::default()
        });
        candidate_species.push(species);
        let end = cursor + options.population;
        engine.spawn_species_batch(
            dna,
            species,
            positions[cursor..end].iter().copied(),
            options.initial_energy,
        )?;
        cursor = end;
    }

    if let Some(path) = &options.vegetable_bot {
        let dna = read_dna(path)?;
        warnings.extend(dna.compatibility_warnings().iter().cloned());
        let species = engine.register_species(SpeciesDefinition {
            name: file_name(path),
            vegetable: true,
            minimum_population: 0,
            reseed: false,
            mutation_rate: 0.0,
            ..SpeciesDefinition::default()
        });
        engine.spawn_species_batch(
            dna,
            species,
            positions[cursor..].iter().copied(),
            options.initial_energy,
        )?;
    }

    warnings.sort();
    warnings.dedup();
    let mut candidate_tracker = CandidateTracker::new(engine.snapshot(), &candidate_species);
    let started = Instant::now();
    advance(&mut engine, options.ticks, &mut candidate_tracker)?;
    let elapsed_seconds = started.elapsed().as_secs_f64();
    let snapshot = engine.snapshot();
    let stats = &snapshot.stats;
    let candidate_final_population = candidate_tracker.population();
    let vegetable_final_population = stats.population.saturating_sub(candidate_final_population);

    Ok(json!({
        "loaded": true,
        "tick": snapshot.tick,
        "ticks": options.ticks,
        "starting_population": starting_population,
        "candidate_starting_population": candidate_starting_population,
        "vegetable_starting_population": options.vegetable_population,
        "final_population": stats.population,
        "candidate_final_population": candidate_final_population,
        "vegetable_final_population": vegetable_final_population,
        "population": stats.population,
        "births": stats.births,
        "deaths": stats.deaths,
        "candidate_births": candidate_tracker.births,
        "candidate_deaths": candidate_tracker.deaths,
        "intentional_movement_events": stats.intentional_movement_events,
        "shots_fired": stats.shots_fired,
        "projectile_impacts": stats.projectile_impacts,
        "feeding_events": stats.feeding_events,
        "energy_harvested": stats.energy_harvested,
        "energy_donated": stats.energy_donated,
        "total_energy": stats.total_energy,
        "compatibility_warnings": warnings,
        "backend": format!("{:?}", engine.backend()),
        "elapsed_seconds": elapsed_seconds,
    }))
}

fn read_dna(path: &Path) -> Result<LegacyDna, Box<dyn Error>> {
    Ok(LegacyDna::parse(&fs::read_to_string(path)?)?)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Bot")
        .to_owned()
}

fn grid_positions(count: usize, width: f32, height: f32) -> Vec<[f32; 2]> {
    let columns = (count as f64).sqrt().ceil().max(1.0) as usize;
    let rows = count.div_ceil(columns).max(1);
    (0..count)
        .map(|index| {
            let column = index % columns;
            let row = index / columns;
            [
                width * (column + 1) as f32 / (columns + 1) as f32,
                height * (row + 1) as f32 / (rows + 1) as f32,
            ]
        })
        .collect()
}

fn advance(
    engine: &mut Engine,
    ticks: u64,
    candidate_tracker: &mut CandidateTracker,
) -> Result<(), Box<dyn Error>> {
    for _ in 0..ticks {
        engine.tick()?;
        candidate_tracker.observe(engine.snapshot());
    }
    Ok(())
}

struct CandidateTracker {
    species: HashSet<SpeciesId>,
    live: HashSet<OrganismId>,
    births: u64,
    deaths: u64,
}

impl CandidateTracker {
    fn new(snapshot: &Snapshot, species: &[SpeciesId]) -> Self {
        let species = species.iter().copied().collect();
        let live = candidate_ids(snapshot, &species);
        Self {
            species,
            live,
            births: 0,
            deaths: 0,
        }
    }

    fn observe(&mut self, snapshot: &Snapshot) {
        let current = candidate_ids(snapshot, &self.species);
        self.births = self
            .births
            .saturating_add(current.difference(&self.live).count() as u64);
        self.deaths = self
            .deaths
            .saturating_add(self.live.difference(&current).count() as u64);
        self.live = current;
    }

    fn population(&self) -> usize {
        self.live.len()
    }
}

fn candidate_ids(snapshot: &Snapshot, species: &HashSet<SpeciesId>) -> HashSet<OrganismId> {
    snapshot
        .organisms
        .iter()
        .filter(|organism| species.contains(&organism.species))
        .map(|organism| organism.id)
        .collect()
}
