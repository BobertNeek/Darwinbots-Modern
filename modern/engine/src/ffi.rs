use crate::persistence::SaveFile;
use crate::world::NativeProcessCaches;
use crate::{Engine, EngineConfig, EngineError, LegacyOrganismFile, OrganismId, SpeciesDefinition};
use serde::Deserialize;
use std::{cell::RefCell, ptr, slice};

pub const ABI_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum DbStatus {
    Ok = 0,
    NullPointer = 1,
    InvalidUtf8 = 2,
    InvalidConfig = 3,
    EngineError = 4,
    InvalidCommand = 5,
}

#[derive(Debug)]
#[repr(C)]
pub struct DbBuffer {
    pub data: *mut u8,
    pub len: usize,
    capacity: usize,
}

#[derive(Deserialize)]
struct CommandBatch {
    version: u32,
    commands: Vec<EngineCommand>,
}

#[derive(Deserialize)]
struct SpeciesMutationRateCommand {
    species: u32,
    rate: f32,
}

#[derive(Deserialize)]
struct OrganismIdCommand {
    slot: u32,
    generation: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum ManualReproductionMutationMode {
    Preserve,
    Apply,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum EngineCommand {
    Tick {
        count: u32,
    },
    SetNativeRestartSeed {
        seed: u64,
        #[serde(default)]
        skin_timer_seconds: Option<[f32; 2]>,
    },
    ReadNativeGaussianSpare,
    SetNativeGaussianSpare {
        spare: Option<f32>,
    },
    ReadNativeSunChange,
    SetNativeSunChange {
        sun_change: u8,
    },
    ReadNativeProcessCaches,
    SetNativeProcessCaches {
        caches: NativeProcessCaches,
    },
    ConfigureLocalMode {
        config: crate::LocalModeConfig,
    },
    ConfigureGraphSampling {
        settings: crate::GraphSamplingSettings,
    },
    UpdateCostSettings {
        costs: crate::DnaCosts,
    },
    UpdateAdvancedMutationSettings {
        settings: crate::AdvancedMutationSettings,
    },
    UpdateDisqualificationRules {
        level: u8,
    },
    UpdateRecognitionFudging {
        mode: u8,
    },
    UpdateMainRules {
        body_cutoff: i32,
        greedy_reproduction: bool,
    },
    UpdateRecordingSettings {
        settings: crate::RecordingSettings,
    },
    StartLocalMode,
    RestartLocalMode,
    AdjudicateLocalRound {
        winner: Option<crate::SpeciesId>,
        reason: String,
    },
    ImportDna {
        dna: String,
        position: Option<[f32; 2]>,
    },
    ImportOrganism {
        payload: Vec<u8>,
        #[serde(default)]
        position: Option<[f32; 2]>,
    },
    Remove {
        slot: u32,
        generation: u32,
    },
    RemoveMany {
        organisms: Vec<OrganismIdCommand>,
    },
    ImportSpecies {
        dna: String,
        name: String,
        vegetable: bool,
        color: u32,
        minimum_population: usize,
        reseed: bool,
        #[serde(default)]
        mutation_rate: f32,
        #[serde(default)]
        mutation_profile: Option<crate::StandardMutationProfile>,
        initial_energy: i32,
        #[serde(default)]
        initial_count: usize,
        #[serde(default)]
        placement: Option<serde_json::Value>,
        #[serde(default)]
        skin: Option<[crate::SkinPoint; 4]>,
        #[serde(default)]
        restrictions: Option<crate::RestrictionColumn>,
        positions: Vec<[f32; 2]>,
    },
    InsertSpecies {
        name: String,
        dna: String,
        position: [f32; 2],
        initial_energy: i32,
    },
    MoveOrganism {
        slot: u32,
        generation: u32,
        position: [f32; 2],
    },
    CloneOrganism {
        slot: u32,
        generation: u32,
        position: [f32; 2],
    },
    ReplaceDna {
        slot: u32,
        generation: u32,
        dna: String,
    },
    ExportDna {
        slot: u32,
        generation: u32,
    },
    ExportSpeciesTemplateDna {
        species: u32,
    },
    ExportOrganismGroup {
        organisms: Vec<OrganismIdCommand>,
    },
    ManualReproduce {
        first_slot: u32,
        first_generation: u32,
        second_slot: Option<u32>,
        second_generation: Option<u32>,
        position: [f32; 2],
        #[serde(default)]
        energy: Option<i32>,
        #[serde(default)]
        body: Option<i32>,
        #[serde(default)]
        mutation_mode: Option<ManualReproductionMutationMode>,
        #[serde(default)]
        focus: bool,
    },
    SwitchBackend {
        backend: crate::BackendPreference,
    },
    AddObstacle {
        id: u32,
        minimum: [f32; 2],
        maximum: [f32; 2],
    },
    UpdateObstacle {
        obstacle: crate::Obstacle,
    },
    RemoveObstacle {
        id: u32,
    },
    AddTeleporter {
        #[serde(flatten)]
        teleporter: crate::Teleporter,
    },
    UpdateTeleporter {
        #[serde(flatten)]
        teleporter: crate::Teleporter,
    },
    RemoveTeleporter {
        id: u32,
    },
    AddFileTeleporter {
        teleporter: crate::FileTeleporter,
    },
    UpdateFileTeleporter {
        teleporter: crate::FileTeleporter,
    },
    RemoveFileTeleporter {
        id: u32,
    },
    SetBrownianMotion {
        value: f32,
    },
    UpdateMutationSettings {
        enabled: bool,
        global_multiplier: f32,
        species_rates: Vec<SpeciesMutationRateCommand>,
        #[serde(default)]
        profile: Option<crate::StandardMutationProfile>,
    },
    UpdateOrganismMutationProfile {
        slot: u32,
        generation: u32,
        profile: crate::StandardMutationProfile,
    },
    SetTiesEnabled {
        enabled: bool,
    },
    SetSpeciesColor {
        species: u32,
        color: u32,
    },
    MakeNewSpecies {
        slot: u32,
        generation: u32,
        name: String,
        color: u32,
    },
    UpdateLegacyRestrictions {
        robot_fixing_allowed: bool,
        asexual_reproduction_allowed: bool,
    },
    ConfigurePlayerBot {
        enabled: bool,
        focus: Option<OrganismIdCommand>,
        bindings: Vec<crate::PlayerBotBinding>,
    },
    SetPlayerBotKey {
        slot: u32,
        generation: u32,
        key: u32,
        pressed: bool,
    },
    SetOrganismMemory {
        slot: u32,
        generation: u32,
        address: u16,
        value: i32,
    },
    ResolveMemoryAddress {
        name: String,
    },
    ExecuteRobots,
    FindBestRobot {
        #[serde(default)]
        energy_weighting: bool,
        #[serde(default)]
        weight_percent: Option<u16>,
    },
    SetOrganismEnergy {
        slot: u32,
        generation: u32,
        energy: i32,
    },
    UpdateRestrictionOptions {
        options: crate::RestrictionOptions,
    },
    SetOrganismAim {
        slot: u32,
        generation: u32,
        aim: i32,
    },
    UpdateEnvironment {
        metabolism_cost: i32,
        vegetable_energy_per_tick: i32,
        sunlight_energy: i32,
        gravity: [f32; 2],
        drag: f32,
        brownian_motion: f32,
        #[serde(default)]
        physics: Option<crate::PhysicsSettings>,
        #[serde(default)]
        shots: Option<crate::ShotSettings>,
        #[serde(default)]
        vegetation: Option<crate::VegetationSettings>,
        #[serde(default)]
        corpse_decay: Option<crate::CorpseDecaySettings>,
        #[serde(default)]
        waste: Option<crate::WasteSettings>,
        #[serde(default)]
        recording: Option<crate::RecordingSettings>,
        #[serde(default)]
        auto_speciation: Option<bool>,
        #[serde(default)]
        speciation_genetic_distance_percent: Option<f32>,
        #[serde(default)]
        toroidal_world: Option<bool>,
        #[serde(default)]
        axis_topology: Option<[bool; 2]>,
        #[serde(default)]
        shapes_visible: Option<bool>,
    },
}

impl Default for DbBuffer {
    fn default() -> Self {
        Self {
            data: ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }
}

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn db_api_version() -> u32 {
    ABI_VERSION
}

// ABI v1 keeps this C entry point safe while retaining its pointer contract.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_backend(engine: *const Engine) -> i32 {
    let Some(engine) = (unsafe { engine.as_ref() }) else {
        return -1;
    };
    match engine.backend() {
        crate::BackendKind::Cpu => 0,
        crate::BackendKind::Gpu => 1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn db_engine_create(
    config_data: *const u8,
    config_len: usize,
    out_engine: *mut *mut Engine,
) -> DbStatus {
    ffi_guard("create engine", || {
        db_engine_create_impl(config_data, config_len, out_engine)
    })
}

fn db_engine_create_impl(
    config_data: *const u8,
    config_len: usize,
    out_engine: *mut *mut Engine,
) -> DbStatus {
    if out_engine.is_null() || (config_data.is_null() && config_len != 0) {
        return DbStatus::NullPointer;
    }
    let config = if config_len == 0 {
        EngineConfig::default()
    } else {
        let bytes = unsafe { slice::from_raw_parts(config_data, config_len) };
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) => return fail(DbStatus::InvalidUtf8, error),
        };
        match serde_json::from_str(text) {
            Ok(config) => config,
            Err(error) => return fail(DbStatus::InvalidConfig, error),
        }
    };
    match Engine::new_native(config) {
        Ok(engine) => {
            unsafe { out_engine.write(Box::into_raw(Box::new(engine))) };
            DbStatus::Ok
        }
        Err(error) => fail(DbStatus::EngineError, error),
    }
}

// ABI v1 keeps this C entry point safe while retaining its pointer contract.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_destroy(engine: *mut Engine) -> DbStatus {
    if engine.is_null() {
        return DbStatus::NullPointer;
    }
    unsafe { drop(Box::from_raw(engine)) };
    DbStatus::Ok
}

#[unsafe(no_mangle)]
pub extern "C" fn db_engine_tick(engine: *mut Engine) -> DbStatus {
    ffi_guard("tick engine", || db_engine_tick_impl(engine))
}

fn db_engine_tick_impl(engine: *mut Engine) -> DbStatus {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return DbStatus::NullPointer;
    };
    match engine.tick() {
        Ok(()) => DbStatus::Ok,
        Err(error) => fail(DbStatus::EngineError, error),
    }
}

// ABI v1 keeps this C entry point safe while retaining its pointer contract.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_import_dna(
    engine: *mut Engine,
    dna_data: *const u8,
    dna_len: usize,
) -> DbStatus {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return DbStatus::NullPointer;
    };
    if dna_data.is_null() && dna_len != 0 {
        return DbStatus::NullPointer;
    }
    let bytes = if dna_len == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(dna_data, dna_len) }
    };
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => return fail(DbStatus::InvalidUtf8, error),
    };
    let dna = match engine.load_dna_source(text) {
        Ok(dna) => dna,
        Err(error) => return fail(DbStatus::EngineError, error),
    };
    match engine.spawn(dna) {
        Ok(_) => DbStatus::Ok,
        Err(error) => fail(DbStatus::EngineError, error),
    }
}

// ABI v1 keeps this C entry point safe while retaining its pointer contract.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_snapshot_json(
    engine: *const Engine,
    out_buffer: *mut DbBuffer,
) -> DbStatus {
    let (Some(engine), Some(out_buffer)) =
        (unsafe { engine.as_ref() }, unsafe { out_buffer.as_mut() })
    else {
        return DbStatus::NullPointer;
    };
    if !out_buffer.data.is_null() {
        return fail(DbStatus::EngineError, "output buffer must be empty");
    }
    match serde_json::to_vec(engine.snapshot()) {
        Ok(mut bytes) => {
            out_buffer.data = bytes.as_mut_ptr();
            out_buffer.len = bytes.len();
            out_buffer.capacity = bytes.capacity();
            std::mem::forget(bytes);
            DbStatus::Ok
        }
        Err(error) => fail(DbStatus::EngineError, error),
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_configuration_json(
    engine: *const Engine,
    out_buffer: *mut DbBuffer,
) -> DbStatus {
    let (Some(engine), Some(out_buffer)) =
        (unsafe { engine.as_ref() }, unsafe { out_buffer.as_mut() })
    else {
        return DbStatus::NullPointer;
    };
    if !out_buffer.data.is_null() {
        return fail(DbStatus::EngineError, "output buffer must be empty");
    }
    match engine
        .configuration_view()
        .and_then(|value| serde_json::to_vec(&value).map_err(Into::into))
    {
        Ok(bytes) => write_buffer(out_buffer, bytes),
        Err(error) => fail(DbStatus::EngineError, error),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn db_engine_command_batch(
    engine: *mut Engine,
    command_data: *const u8,
    command_len: usize,
    out_buffer: *mut DbBuffer,
) -> DbStatus {
    ffi_guard("execute command batch", || {
        db_engine_command_batch_impl(engine, command_data, command_len, out_buffer)
    })
}

fn db_engine_command_batch_impl(
    engine: *mut Engine,
    command_data: *const u8,
    command_len: usize,
    out_buffer: *mut DbBuffer,
) -> DbStatus {
    let (Some(engine), Some(out_buffer)) =
        (unsafe { engine.as_mut() }, unsafe { out_buffer.as_mut() })
    else {
        return DbStatus::NullPointer;
    };
    if command_data.is_null() || command_len == 0 {
        return fail(DbStatus::InvalidCommand, "command batch is empty");
    }
    if !out_buffer.data.is_null() {
        return fail(DbStatus::EngineError, "output buffer must be empty");
    }
    let bytes = unsafe { slice::from_raw_parts(command_data, command_len) };
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => return fail(DbStatus::InvalidUtf8, error),
    };
    let batch: CommandBatch = match serde_json::from_str(text) {
        Ok(batch) => batch,
        Err(error) => return fail(DbStatus::InvalidCommand, error),
    };
    if batch.version != ABI_VERSION {
        return fail(
            DbStatus::InvalidCommand,
            format!("unsupported command batch version {}", batch.version),
        );
    }
    // Pure queries must not reconstruct the world or its GPU backend.
    if !batch.commands.is_empty()
        && batch.commands.iter().all(|command| {
            matches!(
                command,
                EngineCommand::ReadNativeGaussianSpare
                    | EngineCommand::ReadNativeSunChange
                    | EngineCommand::ReadNativeProcessCaches
                    | EngineCommand::ExportSpeciesTemplateDna { .. }
            )
        })
    {
        let results: Result<Vec<_>, EngineError> = batch
            .commands
            .iter()
            .map(|command| match command {
                EngineCommand::ReadNativeGaussianSpare => {
                    Ok(serde_json::json!(engine.read_native_gaussian_spare()))
                }
                EngineCommand::ReadNativeSunChange => {
                    Ok(serde_json::json!(engine.read_native_sun_change()))
                }
                EngineCommand::ReadNativeProcessCaches => {
                    Ok(serde_json::json!(engine.read_native_process_caches()))
                }
                EngineCommand::ExportSpeciesTemplateDna { species } => engine
                    .export_species_template_dna(crate::SpeciesId(*species))
                    .map(|dna| serde_json::json!({ "dna": dna })),
                _ => unreachable!("pure queries were validated above"),
            })
            .collect();
        let results = match results {
            Ok(results) => results,
            Err(error) => return fail(DbStatus::EngineError, error),
        };
        let response = serde_json::json!({
            "version": ABI_VERSION,
            "tick": engine.snapshot().tick,
            "population": engine.population(),
            "backend": format!("{:?}", engine.backend()),
            "results": results,
        });
        return match serde_json::to_vec(&response) {
            Ok(bytes) => write_buffer(out_buffer, bytes),
            Err(error) => fail(DbStatus::EngineError, error),
        };
    }
    let mut command_results = Vec::with_capacity(batch.commands.len());
    let committed_engine = engine;
    let mut candidate = match SaveFile::encode(committed_engine) {
        Ok(bytes) => match SaveFile::decode(&bytes) {
            Ok(candidate) => candidate,
            Err(error) => return fail(DbStatus::EngineError, error.to_string()),
        },
        Err(error) => return fail(DbStatus::EngineError, error.to_string()),
    };
    let engine = &mut candidate;

    for command in batch.commands {
        let result = match command {
            EngineCommand::SetNativeRestartSeed {
                seed,
                skin_timer_seconds,
            } => engine
                .set_native_restart_inputs(seed, skin_timer_seconds)
                .map(|_| serde_json::Value::Null),
            EngineCommand::ReadNativeGaussianSpare => {
                Ok(serde_json::json!(engine.read_native_gaussian_spare()))
            }
            EngineCommand::SetNativeGaussianSpare { spare } => engine
                .set_native_gaussian_spare(spare)
                .map(|_| serde_json::Value::Null),
            EngineCommand::ReadNativeSunChange => {
                Ok(serde_json::json!(engine.read_native_sun_change()))
            }
            EngineCommand::SetNativeSunChange { sun_change } => {
                engine.set_native_sun_change(sun_change);
                Ok(serde_json::Value::Null)
            }
            EngineCommand::ReadNativeProcessCaches => {
                Ok(serde_json::json!(engine.read_native_process_caches()))
            }
            EngineCommand::SetNativeProcessCaches { caches } => engine
                .set_native_process_caches(caches)
                .map(|_| serde_json::Value::Null),
            EngineCommand::Tick { count } => {
                if count > 1_000_000 {
                    return fail(DbStatus::InvalidCommand, "tick count exceeds batch limit");
                }
                engine.tick_many(count).map(|_| serde_json::Value::Null)
            }
            EngineCommand::ConfigureLocalMode { config } => engine
                .configure_local_mode(config)
                .map(|_| serde_json::Value::Null),
            EngineCommand::ConfigureGraphSampling { settings } => {
                engine.configure_graph_sampling(settings);
                Ok(serde_json::Value::Null)
            }
            EngineCommand::UpdateCostSettings { costs } => engine
                .update_cost_settings(costs)
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateAdvancedMutationSettings { settings } => engine
                .update_advanced_mutation_settings(settings)
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateRecognitionFudging { mode } => engine
                .update_recognition_fudging(mode)
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateDisqualificationRules { level } => engine
                .update_disqualification_rules(level)
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateMainRules {
                body_cutoff,
                greedy_reproduction,
            } => engine
                .update_main_rules(body_cutoff, greedy_reproduction)
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateRecordingSettings { settings } => engine
                .update_recording_settings(settings)
                .map(|_| serde_json::Value::Null),
            EngineCommand::StartLocalMode => {
                engine.start_local_mode().map(|_| serde_json::Value::Null)
            }
            EngineCommand::RestartLocalMode => {
                engine.restart_local_mode().map(|_| serde_json::Value::Null)
            }
            EngineCommand::AdjudicateLocalRound { winner, reason } => engine
                .adjudicate_local_round(winner, reason)
                .map(|_| serde_json::Value::Null),
            EngineCommand::ImportDna { dna, position } => {
                engine.load_dna_source(&dna).and_then(|dna| {
                    let compatibility_warnings = dna.compatibility_warnings().to_vec();
                    engine
                        .spawn_at(dna, position.unwrap_or([0.0; 2]))
                        .map(|id| {
                            serde_json::json!({
                                "slot": id.slot(),
                                "generation": id.generation(),
                                "compatibility_warnings": compatibility_warnings,
                            })
                        })
                })
            }
            EngineCommand::ImportOrganism { payload, position } => {
                if payload.starts_with(b"DB2O") {
                    LegacyOrganismFile::decode(&payload).and_then(|file| {
                        engine
                            .import_legacy_organism_file(&file)
                            .map(|count| serde_json::json!({ "count": count }))
                    })
                } else {
                    let result = if position.is_some() {
                        engine.import_native_organisms(&payload, position)
                    } else {
                        engine.insert_native_organisms(&payload)
                    };
                    result.map(|count| serde_json::json!({ "count": count }))
                }
            }
            EngineCommand::Remove { slot, generation } => engine
                .remove(OrganismId::new(slot, generation))
                .map(|_| serde_json::Value::Null),
            EngineCommand::RemoveMany { organisms } => {
                let ids: Vec<_> = organisms
                    .into_iter()
                    .map(|organism| OrganismId::new(organism.slot, organism.generation))
                    .collect();
                engine.remove_many(&ids).map(|_| serde_json::Value::Null)
            }
            EngineCommand::ImportSpecies {
                dna,
                name,
                vegetable,
                color,
                minimum_population,
                reseed,
                mutation_rate,
                mutation_profile,
                initial_energy,
                initial_count,
                placement,
                skin,
                restrictions,
                positions,
            } => {
                let source_text = dna.clone();
                engine.load_dna_source(&dna).and_then(|dna| {
                    if let Some(profile) = &mutation_profile {
                        profile.validate()?;
                    }
                    let compatibility_warnings = dna.compatibility_warnings().to_vec();
                    let mut definition = SpeciesDefinition {
                        name,
                        vegetable,
                        color,
                        minimum_population,
                        reseed,
                        mutation_rate,
                        mutation_profile,
                        ..SpeciesDefinition::default()
                    };
                    if let Some(skin) = skin {
                        definition.skin = skin;
                        definition.custom_skin = true;
                    }
                    definition.restrictions = restrictions;
                    let mode = placement
                        .as_ref()
                        .and_then(|value| value.get("mode"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("random");
                    if !matches!(mode, "random" | "center" | "fixed") {
                        return Err(EngineError::Invariant(
                            "species placement mode must be random, center or fixed".into(),
                        ));
                    }
                    if let Some(value) = placement.as_ref() {
                        definition.placement_bounds = [
                            value
                                .get("left")
                                .and_then(serde_json::Value::as_f64)
                                .unwrap_or(0.0) as f32,
                            value
                                .get("right")
                                .and_then(serde_json::Value::as_f64)
                                .unwrap_or(1.0) as f32,
                            value
                                .get("top")
                                .and_then(serde_json::Value::as_f64)
                                .unwrap_or(0.0) as f32,
                            value
                                .get("bottom")
                                .and_then(serde_json::Value::as_f64)
                                .unwrap_or(1.0) as f32,
                        ];
                    }
                    definition
                        .validate_placement_bounds()
                        .map_err(EngineError::Invariant)?;
                    let species = engine.register_species(definition);
                    let ids = if placement.is_some() && engine.has_native_random() {
                        let positions = if mode == "random" && initial_count > 0 {
                            vec![[0.0; 2]; initial_count]
                        } else {
                            positions
                        };
                        engine.spawn_species_batch_with_native_placement(
                            dna,
                            species,
                            positions,
                            initial_energy,
                            mode == "random",
                        )
                    } else {
                        engine.spawn_species_batch(dna, species, positions, initial_energy)
                    };
                    ids.and_then(|ids| {
                        engine.retain_species_template_source(species, source_text)?;
                        Ok(serde_json::json!({
                            "species": species.0,
                            "count": ids.len(),
                            "compatibility_warnings": compatibility_warnings,
                        }))
                    })
                })
            }
            EngineCommand::InsertSpecies {
                name,
                dna,
                position,
                initial_energy,
            } => engine.load_dna_source(&dna).and_then(|dna| {
                let warnings = dna.compatibility_warnings().to_vec();
                engine
                    .insert_registered_species_at(&name, dna, position, initial_energy)
                    .map(|(species, _)| {
                        serde_json::json!({
                            "species": species.0,
                            "count": 1,
                            "compatibility_warnings": warnings,
                        })
                    })
            }),
            EngineCommand::MoveOrganism {
                slot,
                generation,
                position,
            } => engine
                .move_organism(OrganismId::new(slot, generation), position)
                .map(|_| serde_json::Value::Null),
            EngineCommand::CloneOrganism {
                slot,
                generation,
                position,
            } => engine
                .clone_organism(OrganismId::new(slot, generation), position)
                .map(|id| serde_json::json!({ "slot": id.slot(), "generation": id.generation() })),
            EngineCommand::ReplaceDna {
                slot,
                generation,
                dna,
            } => engine
                .load_dna_source(&dna)
                .and_then(|dna| engine.replace_dna(OrganismId::new(slot, generation), dna))
                .map(|_| serde_json::Value::Null),
            EngineCommand::ExportDna { slot, generation } => engine
                .export_dna(OrganismId::new(slot, generation))
                .map(|dna| serde_json::json!({ "dna": dna })),
            EngineCommand::ExportSpeciesTemplateDna { species } => engine
                .export_species_template_dna(crate::SpeciesId(species))
                .map(|dna| serde_json::json!({ "dna": dna })),
            EngineCommand::ExportOrganismGroup { organisms } => {
                let members: Vec<_> = organisms
                    .into_iter()
                    .map(|id| OrganismId::new(id.slot, id.generation))
                    .collect();
                engine
                    .export_native_organisms(&members)
                    .map(|bytes| serde_json::json!({ "payload": bytes }))
            }
            EngineCommand::ManualReproduce {
                first_slot,
                first_generation,
                second_slot,
                second_generation,
                position,
                energy,
                body,
                mutation_mode,
                focus: _focus,
            } => {
                let second = second_slot
                    .zip(second_generation)
                    .map(|(slot, generation)| OrganismId::new(slot, generation));
                match (energy, body, mutation_mode) {
                    (None, None, None) => engine
                        .manual_reproduce(
                            OrganismId::new(first_slot, first_generation),
                            second,
                            position,
                        )
                        .map(|id| {
                            serde_json::json!({ "slot": id.slot(), "generation": id.generation() })
                        }),
                    (Some(energy), Some(body), Some(mutation_mode)) => engine
                        .manual_reproduce_with_payload(
                            OrganismId::new(first_slot, first_generation),
                            second,
                            position,
                            energy,
                            body,
                            matches!(mutation_mode, ManualReproductionMutationMode::Apply),
                        )
                        .map(|id| {
                            serde_json::json!({ "slot": id.slot(), "generation": id.generation() })
                        }),
                    _ => Err(crate::EngineError::Invariant(
                        "manual reproduction payload must include energy, body, and mutation_mode together"
                            .to_owned(),
                    )),
                }
            }
            EngineCommand::SwitchBackend { backend } => engine
                .switch_backend(backend)
                .map(|_| serde_json::json!({ "backend": format!("{:?}", engine.backend()) })),
            EngineCommand::AddObstacle {
                id,
                minimum,
                maximum,
            } => engine
                .add_obstacle(crate::Obstacle::new(id, minimum, maximum))
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateObstacle { obstacle } => engine
                .update_obstacle(obstacle)
                .map(|_| serde_json::Value::Null),
            EngineCommand::RemoveObstacle { id } => {
                engine.remove_obstacle(id).map(|_| serde_json::Value::Null)
            }
            EngineCommand::AddTeleporter { teleporter } => engine
                .add_teleporter(teleporter)
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateTeleporter { teleporter } => engine
                .update_teleporter(teleporter)
                .map(|_| serde_json::Value::Null),
            EngineCommand::RemoveTeleporter { id } => engine
                .remove_teleporter(id)
                .map(|_| serde_json::Value::Null),
            EngineCommand::AddFileTeleporter { teleporter } => engine
                .add_file_teleporter(teleporter)
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateFileTeleporter { teleporter } => engine
                .update_file_teleporter(teleporter)
                .map(|_| serde_json::Value::Null),
            EngineCommand::RemoveFileTeleporter { id } => engine
                .remove_file_teleporter(id)
                .map(|_| serde_json::Value::Null),
            EngineCommand::SetBrownianMotion { value } => engine
                .set_brownian_motion(value)
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateMutationSettings {
                enabled,
                global_multiplier,
                species_rates,
                profile,
            } => {
                let rates: Vec<_> = species_rates
                    .into_iter()
                    .map(|value| (crate::SpeciesId(value.species), value.rate))
                    .collect();
                engine
                    .update_mutation_settings(enabled, global_multiplier, &rates)
                    .and_then(|_| {
                        profile.map_or(Ok(()), |profile| {
                            engine.set_default_mutation_profile(profile)
                        })
                    })
                    .map(|_| serde_json::Value::Null)
            }
            EngineCommand::UpdateOrganismMutationProfile {
                slot,
                generation,
                profile,
            } => engine
                .set_mutation_profile(OrganismId::new(slot, generation), profile)
                .map(|_| serde_json::Value::Null),
            EngineCommand::SetTiesEnabled { enabled } => {
                engine.set_ties_enabled(enabled);
                Ok(serde_json::Value::Null)
            }
            EngineCommand::SetSpeciesColor { species, color } => engine
                .set_species_color(crate::SpeciesId(species), color)
                .map(|_| serde_json::Value::Null),
            EngineCommand::MakeNewSpecies {
                slot,
                generation,
                name,
                color,
            } => engine
                .make_new_species(OrganismId::new(slot, generation), name, color)
                .map(|species| serde_json::json!({ "species": species.0 })),
            EngineCommand::UpdateLegacyRestrictions {
                robot_fixing_allowed,
                asexual_reproduction_allowed,
            } => {
                engine
                    .update_legacy_restrictions(robot_fixing_allowed, asexual_reproduction_allowed);
                Ok(serde_json::Value::Null)
            }
            EngineCommand::ConfigurePlayerBot {
                enabled,
                focus,
                bindings,
            } => engine
                .configure_player_bot(
                    enabled,
                    focus.map(|value| OrganismId::new(value.slot, value.generation)),
                    bindings,
                )
                .map(|_| serde_json::Value::Null),
            EngineCommand::SetPlayerBotKey {
                slot,
                generation,
                key,
                pressed,
            } => engine
                .set_player_bot_key(OrganismId::new(slot, generation), key, pressed)
                .map(|_| serde_json::Value::Null),
            EngineCommand::SetOrganismMemory {
                slot,
                generation,
                address,
                value,
            } => engine
                .set_organism_memory(OrganismId::new(slot, generation), address, value)
                .map(|_| serde_json::Value::Null),
            EngineCommand::ResolveMemoryAddress { name } => {
                let address = name
                    .parse::<u16>()
                    .ok()
                    .map(i32::from)
                    .or_else(|| crate::sysvar_address(name.trim_start_matches('.')));
                match address.filter(|address| (1..=1000).contains(address)) {
                    Some(address) => Ok(serde_json::json!({ "address": address })),
                    None => Err(crate::EngineError::Invariant(format!(
                        "unknown memory address {name}"
                    ))),
                }
            }
            EngineCommand::ExecuteRobots => {
                engine.execute_robots().map(|_| serde_json::Value::Null)
            }
            EngineCommand::FindBestRobot {
                energy_weighting,
                weight_percent,
            } => {
                let weight = weight_percent.unwrap_or(if energy_weighting { 100 } else { 0 });
                if weight > 200 {
                    return fail(
                        DbStatus::InvalidCommand,
                        "Find Best weighting must be 0..200",
                    );
                }
                Ok(engine.find_best_robot_weighted(weight).map_or(
                    serde_json::Value::Null,
                    |id| serde_json::json!({ "slot": id.slot(), "generation": id.generation() }),
                ))
            }
            EngineCommand::SetOrganismEnergy {
                slot,
                generation,
                energy,
            } => engine
                .set_organism_energy(OrganismId::new(slot, generation), energy)
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateRestrictionOptions { options } => engine
                .update_restriction_options(options)
                .map(|_| serde_json::Value::Null),
            EngineCommand::SetOrganismAim {
                slot,
                generation,
                aim,
            } => engine
                .set_organism_aim(OrganismId::new(slot, generation), aim)
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateEnvironment {
                metabolism_cost,
                vegetable_energy_per_tick,
                sunlight_energy,
                gravity,
                drag,
                brownian_motion,
                physics,
                shots,
                vegetation,
                corpse_decay,
                waste,
                recording,
                auto_speciation,
                speciation_genetic_distance_percent,
                toroidal_world,
                axis_topology,
                shapes_visible,
            } => engine
                .update_environment(
                    metabolism_cost,
                    vegetable_energy_per_tick,
                    sunlight_energy,
                    gravity,
                    drag,
                    brownian_motion,
                )
                .and_then(|_| {
                    engine.update_db2_settings(
                        physics,
                        shots,
                        vegetation,
                        corpse_decay,
                        waste,
                        recording,
                    )
                })
                .and_then(|_| {
                    engine.update_speciation_settings(
                        auto_speciation,
                        speciation_genetic_distance_percent,
                    )
                })
                .map(|_| {
                    if let Some(enabled) = toroidal_world {
                        engine.set_toroidal_world(enabled);
                    }
                    if let Some(wrapped_axes) = axis_topology {
                        engine.set_axis_topology(wrapped_axes);
                    }
                    if let Some(visible) = shapes_visible {
                        engine.set_shapes_visible(visible);
                    }
                })
                .map(|_| serde_json::Value::Null),
        };
        match result {
            Ok(value) => command_results.push(value),
            Err(error) => return fail(DbStatus::EngineError, error),
        }
    }
    let result = serde_json::json!({
        "version": ABI_VERSION,
        "tick": engine.snapshot().tick,
        "population": engine.population(),
        "backend": format!("{:?}", engine.backend()),
        "results": command_results,
    });
    let bytes = match serde_json::to_vec(&result) {
        Ok(bytes) => bytes,
        Err(error) => return fail(DbStatus::EngineError, error),
    };
    let status = write_buffer(out_buffer, bytes);
    if status == DbStatus::Ok {
        *committed_engine = candidate;
    }
    status
}

// ABI v1 keeps this C entry point safe while retaining its pointer contract.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_capabilities_json(
    engine: *const Engine,
    out_buffer: *mut DbBuffer,
) -> DbStatus {
    let (Some(engine), Some(out_buffer)) =
        (unsafe { engine.as_ref() }, unsafe { out_buffer.as_mut() })
    else {
        return DbStatus::NullPointer;
    };
    if !out_buffer.data.is_null() {
        return fail(DbStatus::EngineError, "output buffer must be empty");
    }
    let capabilities = engine.capabilities();
    let result = serde_json::json!({
        "version": ABI_VERSION,
        "active": format!("{:?}", capabilities.active),
        "gpu_available": capabilities.gpu_available,
        "fallback_reason": capabilities.fallback_reason,
    });
    match serde_json::to_vec(&result) {
        Ok(bytes) => write_buffer(out_buffer, bytes),
        Err(error) => fail(DbStatus::EngineError, error),
    }
}

// ABI v1 keeps this C entry point safe while retaining its pointer contract.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_save(engine: *const Engine, out_buffer: *mut DbBuffer) -> DbStatus {
    let (Some(engine), Some(out_buffer)) =
        (unsafe { engine.as_ref() }, unsafe { out_buffer.as_mut() })
    else {
        return DbStatus::NullPointer;
    };
    if !out_buffer.data.is_null() {
        return fail(DbStatus::EngineError, "output buffer must be empty");
    }
    match SaveFile::encode(engine) {
        Ok(bytes) => write_buffer(out_buffer, bytes),
        Err(error) => fail(DbStatus::EngineError, error),
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_save_legacy(
    engine: *const Engine,
    out_buffer: *mut DbBuffer,
) -> DbStatus {
    let (Some(engine), Some(out_buffer)) =
        (unsafe { engine.as_ref() }, unsafe { out_buffer.as_mut() })
    else {
        return DbStatus::NullPointer;
    };
    if !out_buffer.data.is_null() {
        return fail(DbStatus::EngineError, "output buffer must be empty");
    }
    match engine.export_native_sim() {
        Ok(bytes) => write_buffer(out_buffer, bytes),
        Err(error) => fail(DbStatus::EngineError, error),
    }
}

// ABI v1 keeps this C entry point safe while retaining its pointer contract.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_save_without_mutations(
    engine: *const Engine,
    out_buffer: *mut DbBuffer,
) -> DbStatus {
    ffi_guard("save engine without mutations", || {
        let (Some(engine), Some(out_buffer)) =
            (unsafe { engine.as_ref() }, unsafe { out_buffer.as_mut() })
        else {
            return DbStatus::NullPointer;
        };
        if !out_buffer.data.is_null() {
            return fail(DbStatus::EngineError, "output buffer must be empty");
        }
        match SaveFile::encode_without_mutations(engine) {
            Ok(bytes) => write_buffer(out_buffer, bytes),
            Err(error) => fail(DbStatus::EngineError, error),
        }
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn db_engine_load(
    engine: *mut Engine,
    save_data: *const u8,
    save_len: usize,
) -> DbStatus {
    ffi_guard("load engine", || {
        db_engine_load_impl(engine, save_data, save_len)
    })
}

fn db_engine_load_impl(engine: *mut Engine, save_data: *const u8, save_len: usize) -> DbStatus {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return DbStatus::NullPointer;
    };
    if save_data.is_null() || save_len == 0 {
        return fail(DbStatus::EngineError, "save buffer is empty");
    }
    let bytes = unsafe { slice::from_raw_parts(save_data, save_len) };
    match SaveFile::decode(bytes) {
        Ok(mut loaded) => {
            // Native .sim omits Global.gset preferences. DB3S instead restores
            // its complete saved policy; the two load formats stay distinct.
            if !bytes.starts_with(b"DB3S") {
                loaded.retain_native_global_preferences(engine.configuration());
                loaded.inherit_native_gaussian_spare(engine);
                if let Err(error) = loaded.inherit_native_process_caches(engine) {
                    return fail(DbStatus::EngineError, error);
                }
            }
            *engine = loaded;
            DbStatus::Ok
        }
        Err(error) => fail(DbStatus::EngineError, error),
    }
}

// ABI v1 keeps this C entry point safe while retaining its pointer contract.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_last_error_json(out_buffer: *mut DbBuffer) -> DbStatus {
    let Some(out_buffer) = (unsafe { out_buffer.as_mut() }) else {
        return DbStatus::NullPointer;
    };
    if !out_buffer.data.is_null() {
        return DbStatus::EngineError;
    }
    let message = LAST_ERROR.with(|last_error| last_error.borrow().clone());
    match serde_json::to_vec(&serde_json::json!({ "version": ABI_VERSION, "message": message })) {
        Ok(bytes) => write_buffer(out_buffer, bytes),
        Err(_) => DbStatus::EngineError,
    }
}

// ABI v1 keeps this C entry point safe while retaining its pointer contract.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_buffer_free(buffer: *mut DbBuffer) -> DbStatus {
    let Some(buffer) = (unsafe { buffer.as_mut() }) else {
        return DbStatus::NullPointer;
    };
    if !buffer.data.is_null() {
        unsafe {
            drop(Vec::from_raw_parts(
                buffer.data,
                buffer.len,
                buffer.capacity,
            ))
        };
    }
    *buffer = DbBuffer::default();
    DbStatus::Ok
}

fn fail(status: DbStatus, error: impl ToString) -> DbStatus {
    LAST_ERROR.with(|last_error| *last_error.borrow_mut() = error.to_string());
    status
}

fn write_buffer(out_buffer: &mut DbBuffer, mut bytes: Vec<u8>) -> DbStatus {
    out_buffer.data = bytes.as_mut_ptr();
    out_buffer.len = bytes.len();
    out_buffer.capacity = bytes.capacity();
    std::mem::forget(bytes);
    DbStatus::Ok
}

fn ffi_guard(operation: &str, action: impl FnOnce() -> DbStatus) -> DbStatus {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)) {
        Ok(status) => status,
        Err(payload) => {
            let detail = payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("unknown native panic");
            fail(
                DbStatus::EngineError,
                format!("{operation} panicked: {detail}"),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LegacyDna, SpeciesId};

    fn native_cache_fixture(
        light_availability: f64,
        displayed: i32,
        current_index: u8,
        earlier_energy: i32,
        shots_this_cycle: u64,
    ) -> NativeProcessCaches {
        let mut ring = vec![0_i32; 101];
        ring[usize::from(current_index)] = 555;
        ring[(usize::from(current_index) + 98) % 100] = earlier_energy;
        ring[100] = -197;
        serde_json::from_value(serde_json::json!({
            "light_availability": light_availability,
            "shots_this_cycle": shots_this_cycle,
            "energy": {
                "ring": ring,
                "current_index": current_index,
                "displayed": displayed,
            }
        }))
        .unwrap()
    }

    #[test]
    fn native_sim_load_inherits_process_light_while_db3s_restores_saved_light() {
        let mut saved = Engine::new_native(EngineConfig::testing()).unwrap();
        saved
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0, 500.0])
            .unwrap();
        let saved_caches = native_cache_fixture(0.75, 1_007, 6, 900, 21);
        let saved_context = serde_json::to_value(&saved_caches).unwrap();
        saved.set_native_process_caches(saved_caches).unwrap();
        let original_bytes = saved.export_native_sim().unwrap();
        let fresh_process = Engine::from_native_sim(&original_bytes).unwrap();
        assert_eq!(fresh_process.native_light_availability(), 0.0);
        let fresh_caches =
            serde_json::to_value(fresh_process.read_native_process_caches()).unwrap();
        assert_eq!(fresh_caches["shots_this_cycle"], 0);
        assert_eq!(fresh_caches["energy"]["displayed"], 0);
        assert_eq!(fresh_caches["energy"]["current_index"], 0);
        assert_eq!(
            fresh_caches["energy"]["ring"],
            serde_json::json!(vec![0_i32; 101])
        );

        let mut current = Engine::new_native(EngineConfig::testing()).unwrap();
        let current_caches = native_cache_fixture(0.25, 109, 7, 37, 17);
        let current_context = serde_json::to_value(&current_caches).unwrap();
        current.set_native_process_caches(current_caches).unwrap();
        assert_eq!(
            db_engine_load(&mut current, original_bytes.as_ptr(), original_bytes.len()),
            DbStatus::Ok
        );
        assert_eq!(current.native_light_availability(), 0.25);
        assert_eq!(
            serde_json::to_value(current.read_native_process_caches()).unwrap(),
            current_context
        );
        assert_eq!(current.snapshot().stats.native_energy_displayed, Some(109));
        assert_eq!(current.snapshot().stats.native_energy_delta, Some(72));
        assert_eq!(current.snapshot().stats.native_shots_visited, Some(17));
        assert_eq!(current.population(), 1);

        let modern_bytes = SaveFile::encode(&saved).unwrap();
        assert_eq!(
            db_engine_load(&mut current, modern_bytes.as_ptr(), modern_bytes.len()),
            DbStatus::Ok
        );
        assert_eq!(current.native_light_availability(), 0.75);
        assert_eq!(
            serde_json::to_value(current.read_native_process_caches()).unwrap(),
            saved_context
        );
        assert_eq!(
            current.snapshot().stats.native_energy_displayed,
            Some(1_007)
        );
        assert_eq!(current.snapshot().stats.native_energy_delta, Some(107));
        assert_eq!(current.snapshot().stats.native_shots_visited, Some(21));
    }

    #[test]
    fn invalid_load_preserves_process_light_and_the_existing_world() {
        let mut current = Engine::new_native(EngineConfig::testing()).unwrap();
        current
            .spawn_at(LegacyDna::parse("stop").unwrap(), [321.0, 654.0])
            .unwrap();
        current
            .set_native_process_caches(native_cache_fixture(0.375, 421, 29, 199, 17))
            .unwrap();
        let before = SaveFile::encode(&current).unwrap();
        let invalid = b"invalid simulation";
        assert_ne!(
            db_engine_load(&mut current, invalid.as_ptr(), invalid.len()),
            DbStatus::Ok
        );
        assert_eq!(current.native_light_availability(), 0.375);
        assert_eq!(SaveFile::encode(&current).unwrap(), before);
    }

    #[test]
    fn process_cache_queries_leave_saved_world_rng_and_current_snapshot_unchanged() {
        let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
        engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [321.0, 654.0])
            .unwrap();
        engine.set_native_gaussian_spare(Some(1.25)).unwrap();
        engine.set_native_sun_change(22);
        let caches = native_cache_fixture(0.375, 421, 29, 199, 17);
        let expected = serde_json::to_value(&caches).unwrap();
        engine.set_native_process_caches(caches).unwrap();
        let before = SaveFile::encode(&engine).unwrap();
        for _ in 0..2 {
            let (status, response) = classic_test_batch(
                &mut engine,
                serde_json::json!([
                    {"type":"read_native_gaussian_spare"},
                    {"type":"read_native_sun_change"},
                    {"type":"read_native_process_caches"}
                ]),
            );
            assert_eq!(status, DbStatus::Ok);
            let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
            assert_eq!(response["results"][0], 1.25);
            assert_eq!(response["results"][1], 22);
            assert_eq!(response["results"][2], expected);
            assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        }
    }

    #[test]
    fn process_cache_command_updates_status_without_advancing_the_world_or_rng() {
        let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
        engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [321.0, 654.0])
            .unwrap();
        engine.set_native_gaussian_spare(Some(1.25)).unwrap();
        engine.set_native_sun_change(22);
        let before = serde_json::to_value(&engine).unwrap();
        let snapshot_before = engine.snapshot().clone();
        let expected = serde_json::to_value(native_cache_fixture(0.625, 109, 7, 37, 17)).unwrap();
        let (status, response) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"set_native_process_caches","caches":expected},
                {"type":"read_native_process_caches"}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert!(response["results"][0].is_null());
        assert_eq!(response["results"][1], expected);
        let after = serde_json::to_value(&engine).unwrap();
        assert_eq!(after["native_random"], before["native_random"]);
        assert_eq!(after["slots"], before["slots"]);
        assert_eq!(after["config"], before["config"]);
        assert_eq!(engine.snapshot().tick, snapshot_before.tick);
        assert_eq!(engine.snapshot().native_cycle, snapshot_before.native_cycle);
        assert_eq!(engine.snapshot().organisms, snapshot_before.organisms);
        assert_eq!(engine.read_native_sun_change(), 22);
        assert_eq!(engine.snapshot().stats.native_energy_displayed, Some(109));
        assert_eq!(engine.snapshot().stats.native_energy_delta, Some(72));
        assert_eq!(engine.snapshot().stats.native_shots_visited, Some(17));
        let mut zero_shots = expected.clone();
        zero_shots["shots_this_cycle"] = serde_json::json!(0);
        let (status, _) = classic_test_batch(
            &mut engine,
            serde_json::json!([{"type":"set_native_process_caches","caches":zero_shots}]),
        );
        assert_eq!(status, DbStatus::Ok);
        assert_eq!(engine.snapshot().stats.native_shots_visited, Some(0));
        assert_eq!(engine.snapshot().tick, snapshot_before.tick);
    }

    #[test]
    fn malformed_process_cache_commands_roll_back_the_entire_batch() {
        let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
        let id = engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [321.0, 654.0])
            .unwrap();
        engine
            .set_native_process_caches(native_cache_fixture(0.375, 421, 29, 199, 17))
            .unwrap();
        let valid = serde_json::to_value(native_cache_fixture(0.625, 109, 7, 37, 17)).unwrap();
        let mut invalid_cases = Vec::new();
        for light in [-0.125, 1.125] {
            let mut invalid = valid.clone();
            invalid["light_availability"] = serde_json::json!(light);
            invalid_cases.push(invalid);
        }
        for length in [0, 100, 102] {
            let mut invalid = valid.clone();
            invalid["energy"]["ring"] = serde_json::json!(vec![0_i32; length]);
            invalid_cases.push(invalid);
        }
        for index in [-1, 100, 256] {
            let mut invalid = valid.clone();
            invalid["energy"]["current_index"] = serde_json::json!(index);
            invalid_cases.push(invalid);
        }
        let mut invalid = valid.clone();
        invalid["energy"]["ring"][0] = serde_json::json!(2_147_483_648_i64);
        invalid_cases.push(invalid);
        for count in [serde_json::json!(-1), serde_json::json!(2_147_483_648_u64)] {
            let mut invalid = valid.clone();
            invalid["shots_this_cycle"] = count;
            invalid_cases.push(invalid);
        }
        for invalid in invalid_cases {
            let before = SaveFile::encode(&engine).unwrap();
            let (status, response) = classic_test_batch(
                &mut engine,
                serde_json::json!([
                    {"type":"set_organism_energy","slot":id.slot(),"generation":id.generation(),"energy":42},
                    {"type":"set_native_process_caches","caches":valid},
                    {"type":"set_native_process_caches","caches":invalid}
                ]),
            );
            assert_ne!(status, DbStatus::Ok);
            assert!(response.is_empty());
            assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        }
    }

    #[test]
    fn invalid_find_best_weight_preserves_the_entire_command_batch() {
        let mut engine = Engine::new(EngineConfig::testing()).unwrap();
        let id = engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0, 500.0])
            .unwrap();
        let command = serde_json::to_vec(&serde_json::json!({
            "version": 1,
            "commands": [
                { "type": "set_organism_energy", "slot": id.slot(), "generation": id.generation(), "energy": 42 },
                { "type": "find_best_robot", "weight_percent": 201 }
            ]
        })).unwrap();
        let mut buffer = DbBuffer::default();
        assert_eq!(
            db_engine_command_batch(&mut engine, command.as_ptr(), command.len(), &mut buffer),
            DbStatus::InvalidCommand
        );
        assert_eq!(engine.organism(id).unwrap().energy, 1000);
        assert!(buffer.data.is_null());
    }

    #[test]
    fn legacy_save_emits_native_24832_sim_for_file_command() {
        let mut engine = Engine::new(EngineConfig::testing()).expect("fixture engine");
        engine
            .spawn_at(LegacyDna::parse("stop").expect("fixture DNA"), [12.0, 34.0])
            .expect("fixture organism");
        let mut buffer = DbBuffer::default();

        assert_eq!(
            db_engine_save_legacy(&engine, &mut buffer),
            DbStatus::Ok,
            "legacy file command must return a payload"
        );
        let bytes = unsafe { slice::from_raw_parts(buffer.data, buffer.len) }.to_vec();
        let decoded =
            crate::native_binary::NativeRecordFile::decode(&bytes).expect("native payload");
        assert_eq!(decoded.robots.len(), 1);
        let resumed = Engine::from_native_sim(&bytes).expect("native simulation reloads");
        assert_eq!(resumed.snapshot().organisms.len(), 1);
        assert_eq!(resumed.snapshot().organisms[0].position, [12.0, 34.0]);
        assert_eq!(db_buffer_free(&mut buffer), DbStatus::Ok);
    }

    #[test]
    fn ffi_guard_converts_panics_to_structured_engine_errors() {
        let status = ffi_guard("test operation", || panic!("deliberate failure"));
        assert_eq!(status, DbStatus::EngineError);
        let message = LAST_ERROR.with(|error| error.borrow().clone());
        assert!(message.contains("test operation panicked"));
        assert!(message.contains("deliberate failure"));
    }
    fn source_fixture_engine(native: bool) -> Engine {
        if native {
            return Engine::new_native(EngineConfig::testing()).unwrap();
        }
        // Engine::new already has a native stream. Represent the supported
        // legacy save state explicitly without adding a runtime setting.
        let mut state =
            serde_json::to_value(Engine::new(EngineConfig::testing()).unwrap()).unwrap();
        state["native_random"] = serde_json::Value::Null;
        let engine: Engine = serde_json::from_value(state).unwrap();
        assert!(!engine.has_native_random());
        engine
    }

    fn classic_test_batch(engine: &mut Engine, commands: serde_json::Value) -> (DbStatus, Vec<u8>) {
        let payload =
            serde_json::to_vec(&serde_json::json!({"version":1,"commands":commands})).unwrap();
        let mut buffer = DbBuffer::default();
        let status = db_engine_command_batch(engine, payload.as_ptr(), payload.len(), &mut buffer);
        let result = if buffer.data.is_null() {
            Vec::new()
        } else {
            unsafe { slice::from_raw_parts(buffer.data, buffer.len) }.to_vec()
        };
        if !buffer.data.is_null() {
            assert_eq!(db_buffer_free(&mut buffer), DbStatus::Ok);
        }
        (status, result)
    }

    #[test]
    fn classic_raw_source_and_clock_inputs_survive_public_ffi_clones_saves_and_atomic_failures() {
        let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
        let raw = "' original CRLF input\r\ndef marker 900\r\nstart 7 .marker store stop end 123\r\n'#tag:retained\r\n";
        let (status, _) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"set_native_restart_seed","seed":112,"skin_timer_seconds":[1.125,1.375]},
                {"type":"import_species","name":"Original.txt","dna":raw,"vegetable":false,"color":4279312947u32,"minimum_population":0,"reseed":false,"mutation_rate":0,"initial_energy":777,"initial_count":1,"positions":[[500,500]]}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        let source = engine
            .snapshot()
            .species
            .iter()
            .position(|s| s.name == "Original.txt")
            .unwrap();
        let saved = SaveFile::encode(&engine).unwrap();
        let (status, result) = classic_test_batch(
            &mut engine,
            serde_json::json!([{ "type":"export_species_template_dna","species":source }]),
        );
        assert_eq!(status, DbStatus::Ok);
        let result: serde_json::Value = serde_json::from_slice(&result).unwrap();
        assert_eq!(result["results"][0]["dna"], raw);
        assert_eq!(SaveFile::encode(&engine).unwrap(), saved);
        engine = SaveFile::decode(&saved).unwrap();
        assert_eq!(
            serde_json::to_value(&engine).unwrap()["native_skin_timer_seconds"],
            serde_json::json!([1.125, 1.375])
        );
        let (status, _) = classic_test_batch(
            &mut engine,
            serde_json::json!([{ "type":"set_native_restart_seed","seed":162 }]),
        );
        assert_eq!(status, DbStatus::Ok);
        assert_eq!(
            serde_json::to_value(&engine).unwrap()["native_skin_timer_seconds"],
            serde_json::json!([1.125, 1.375])
        );
        for timers in [
            serde_json::json!([-0.1, 1]),
            serde_json::json!([0, 86400]),
            serde_json::json!([1]),
        ] {
            let before = SaveFile::encode(&engine).unwrap();
            let (status, result) = classic_test_batch(
                &mut engine,
                serde_json::json!([{ "type":"set_native_restart_seed","seed":999,"skin_timer_seconds":timers }]),
            );
            assert_ne!(status, DbStatus::Ok);
            assert!(result.is_empty());
            assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        }
        let before = SaveFile::encode(&engine).unwrap();
        let (status, result) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"set_native_restart_seed","seed":999,"skin_timer_seconds":[5,6]},
                {"type":"export_species_template_dna","species":4294967295u32}
            ]),
        );
        assert_ne!(status, DbStatus::Ok);
        assert!(result.is_empty());
        assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        assert_eq!(
            engine
                .export_species_template_dna(SpeciesId(source as u32))
                .unwrap(),
            raw
        );
    }

    #[test]
    fn native_text_import_insert_replace_keep_source_coding_and_private_context_across_ffi_clones()
    {
        for native in [true, false] {
            let mut engine = source_fixture_engine(native);
            let mut raw = String::from("' exact CRLF source\r\ndef Foo 7\r\ndef foo 9\r\n");
            for index in 0..53 {
                raw.push_str(&format!(
                    "def v{index} {}\r\n",
                    if index == 47 { 32767 } else { 3000 + index }
                ));
            }
            raw.push_str("11 start .Foo .foo 1 999 += stop end 123\r\nNewMove\r\n");
            let expected = engine.load_dna_source(&raw).unwrap();
            assert_eq!(expected.uses_new_move(), !native);
            assert_eq!(
                expected
                    .blocks()
                    .iter()
                    .map(|block| (block.class as u8, block.value))
                    .collect::<Vec<_>>(),
                default_import_literal(native)
            );
            if native {
                assert_eq!(expected.blocks()[0].class, crate::TokenClass::Flow);
                assert_eq!(expected.blocks()[1].value, 7);
                assert_eq!(expected.blocks()[2].value, 9);
            } else {
                assert_eq!(expected.blocks()[0].value, 11);
            }
            let (status, result) = classic_test_batch(
                &mut engine,
                serde_json::json!([{ "type":"import_species","name":"Native Source.txt","dna":raw,"vegetable":false,"color":4279312947u32,"minimum_population":0,"reseed":false,"initial_energy":1000,"initial_count":1,"positions":[[500,500]] }]),
            );
            assert_eq!(status, DbStatus::Ok);
            let response: serde_json::Value = serde_json::from_slice(&result).unwrap();
            let species = SpeciesId(response["results"][0]["species"].as_u64().unwrap() as u32);
            let first = engine.snapshot().organisms[0].id;
            assert_eq!(engine.organism(first).unwrap().flags.new_move, !native);
            assert_eq!(
                engine.organism(first).unwrap().private_variable_count,
                if native { 56 } else { 51 }
            );
            assert_eq!(
                LegacyDna::parse(&engine.export_dna(first).unwrap())
                    .unwrap()
                    .blocks(),
                expected.blocks()
            );
            assert_eq!(engine.export_species_template_dna(species).unwrap(), raw);
            let saved = SaveFile::encode(&engine).unwrap();
            engine = SaveFile::decode(&saved).unwrap();
            assert_eq!(
                engine.organism(first).unwrap().private_variable_count,
                if native { 56 } else { 51 }
            );
            assert_eq!(engine.export_species_template_dna(species).unwrap(), raw);
            if native {
                let record = crate::native_binary::NativeRecordFile::decode(
                    &engine.export_native_sim().unwrap(),
                )
                .unwrap();
                assert_eq!(record.robots[0].word("private_variable_count").unwrap(), 56);
                let variables = record.robots[0].bytes("private_variables").unwrap();
                let mut offset = 0;
                let mut saved_variables = Vec::new();
                for _ in 0..50 {
                    let length =
                        i16::from_le_bytes(variables[offset..offset + 2].try_into().unwrap())
                            as usize;
                    offset += 2;
                    let name =
                        String::from_utf8(variables[offset..offset + length].to_vec()).unwrap();
                    offset += length;
                    let value =
                        i16::from_le_bytes(variables[offset..offset + 2].try_into().unwrap());
                    offset += 2;
                    saved_variables.push((name, value));
                }
                assert_eq!(saved_variables[0], ("Foo".into(), 7));
                assert_eq!(saved_variables[1], ("foo".into(), 9));
                assert_eq!(saved_variables[49], ("v47".into(), 32767));
                let imported = Engine::from_native_sim(&record.encode().unwrap()).unwrap();
                assert_eq!(imported.snapshot().organisms[0].private_variable_count, 56);
            }
            let (status, _) = classic_test_batch(
                &mut engine,
                serde_json::json!([{ "type":"insert_species","name":"Native Source.txt","dna":raw,"position":[321,654],"initial_energy":2000 }]),
            );
            assert_eq!(status, DbStatus::Ok);
            assert_eq!(engine.population(), 2);
            assert!(
                engine
                    .snapshot()
                    .organisms
                    .iter()
                    .all(|r| r.private_variable_count == if native { 56 } else { 51 })
            );
            assert_eq!(engine.export_species_template_dna(species).unwrap(), raw);
            let replacement = "def A 4\r\ndef a 5\r\nstart .A .a += stop end\r\nNewMove\r\n";
            let expected_replacement = engine.load_dna_source(replacement).unwrap();
            let (status, _) = classic_test_batch(
                &mut engine,
                serde_json::json!([{ "type":"replace_dna","slot":first.slot(),"generation":first.generation(),"dna":replacement }]),
            );
            assert_eq!(status, DbStatus::Ok);
            assert_eq!(
                LegacyDna::parse(&engine.export_dna(first).unwrap())
                    .unwrap()
                    .blocks(),
                expected_replacement.blocks()
            );
            assert_eq!(engine.organism(first).unwrap().flags.new_move, !native);
            assert_eq!(
                engine.organism(first).unwrap().private_variable_count,
                if native { 3 } else { 51 }
            );
            assert_eq!(engine.export_species_template_dna(species).unwrap(), raw);
            let (status, result) = classic_test_batch(
                &mut engine,
                serde_json::json!([{ "type":"import_species","name":"Zero Count.txt","dna":raw,"vegetable":false,"color":4279312947u32,"minimum_population":0,"reseed":false,"initial_energy":777,"initial_count":0,"positions":[] }]),
            );
            assert_eq!(status, DbStatus::Ok);
            let result: serde_json::Value = serde_json::from_slice(&result).unwrap();
            let zero = SpeciesId(result["results"][0]["species"].as_u64().unwrap() as u32);
            assert_eq!(engine.population(), 2);
            assert_eq!(engine.export_species_template_dna(zero).unwrap(), raw);
            let resumed = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
            assert_eq!(resumed.export_species_template_dna(zero).unwrap(), raw);
            assert_eq!(
                resumed.organism(first).unwrap().private_variable_count,
                if native { 3 } else { 51 }
            );
        }
    }

    #[test]
    fn invalid_native_text_loader_fails_the_entire_import_batch_without_committing() {
        let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
        let before = SaveFile::encode(&engine).unwrap();
        let (status, result) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"set_native_restart_seed","seed":999},
                {"type":"import_species","name":"Overflow.txt","dna":"start 32768 stop","vegetable":false,"color":4279312947u32,"minimum_population":0,"reseed":false,"initial_energy":1000,"initial_count":1,"positions":[[500,500]]}
            ]),
        );
        assert_ne!(status, DbStatus::Ok);
        assert!(result.is_empty());
        assert_eq!(SaveFile::encode(&engine).unwrap(), before);
    }
    fn default_import_literal(native: bool) -> Vec<(u8, i32)> {
        if native {
            vec![
                (9, 2),
                (0, 7),
                (0, 9),
                (0, 1),
                (0, 999),
                (0, 0),
                (9, 4),
                (10, 1),
            ]
        } else {
            vec![
                (0, 11),
                (9, 2),
                (0, 9),
                (0, 9),
                (0, 1),
                (0, 999),
                (7, 4),
                (9, 4),
                (10, 1),
            ]
        }
    }

    fn default_import_source() -> String {
        let mut source = String::from("' exact CRLF input\r\ndef Foo 7\r\ndef foo 9\r\n");
        for index in 0..53 {
            source.push_str(&format!(
                "def v{index} {}\r\n",
                if index == 47 { 32767 } else { 3000 + index }
            ));
        }
        source.push_str("11 start .Foo .foo 1 999 += stop end 123\r\nNewMove\r\n");
        source
    }

    fn assert_default_source_actor(engine: &Engine, id: OrganismId, native: bool) {
        let dna = engine.dna(id).unwrap();
        assert_eq!(
            dna.blocks()
                .iter()
                .map(|block| (block.class as u8, block.value))
                .collect::<Vec<_>>(),
            default_import_literal(native)
        );
        assert_eq!(dna.uses_new_move(), !native);
        assert_eq!(engine.organism(id).unwrap().flags.new_move, !native);
        assert_eq!(
            engine.organism(id).unwrap().private_variable_count,
            if native { 56 } else { 51 }
        );
        assert_eq!(dna.native_user_variables().len(), 55);
        assert_eq!(
            &dna.native_user_variables()[..2],
            &[("Foo".into(), 7), ("foo".into(), 9)]
        );
        if native {
            // Inspect the persisted runtime cells beyond the native file's50
            // saved slots, independently from the DNA definition list.
            let saved = SaveFile::encode(engine).unwrap();
            let payload: serde_json::Value = serde_json::from_slice(&saved[10..]).unwrap();
            let program = &payload["slots"][id.slot() as usize]["organism"]["program"];
            let cells = program["private_variables"]["cells"].as_array().unwrap();
            let cell = |address| {
                cells
                    .iter()
                    .find(|entry| entry[0].as_u64() == Some(address))
                    .unwrap()[1]
                    .as_i64()
                    .unwrap()
            };
            assert_eq!(cell(50), 32767);
            assert_eq!(cell(51), 3048);
            assert_eq!(cell(55), 3052);
        }
    }

    #[test]
    fn default_utf8_import_boundaries_use_native_source_without_creating_startup_templates() {
        for native in [true, false] {
            for command in [false, true] {
                let mut engine = source_fixture_engine(native);
                let templates = serde_json::to_value(&engine).unwrap()["species_templates"].clone();
                assert!(
                    engine
                        .export_species_template_dna(SpeciesId::default())
                        .is_err()
                );
                let source = default_import_source();
                if command {
                    let (status, result) = classic_test_batch(
                        &mut engine,
                        serde_json::json!([
                            {"type":"import_dna","dna":source,"position":[321,654]}
                        ]),
                    );
                    assert_eq!(status, DbStatus::Ok);
                    let result: serde_json::Value = serde_json::from_slice(&result).unwrap();
                    assert!(result["results"][0]["compatibility_warnings"].is_array());
                } else {
                    assert_eq!(
                        db_engine_import_dna(&mut engine, source.as_ptr(), source.len()),
                        DbStatus::Ok
                    );
                }
                let id = engine.snapshot().organisms[0].id;
                assert_eq!(
                    engine.organism(id).unwrap().position,
                    if command { [321.0, 654.0] } else { [0.0, 0.0] }
                );
                assert_default_source_actor(&engine, id, native);
                assert_eq!(
                    serde_json::to_value(&engine).unwrap()["species_templates"],
                    templates
                );
                assert!(
                    engine
                        .export_species_template_dna(SpeciesId::default())
                        .is_err()
                );
                engine = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
                assert_default_source_actor(&engine, id, native);
                assert!(
                    engine
                        .export_species_template_dna(SpeciesId::default())
                        .is_err()
                );

                // Later individual programs may differ while sharing their
                // default species identity. Neither import becomes a template.
                let second = "def Two 13\r\nstart .Two 999 store stop\r\n";
                assert_eq!(
                    db_engine_import_dna(&mut engine, second.as_ptr(), second.len()),
                    DbStatus::Ok
                );
                let (status, _) = classic_test_batch(
                    &mut engine,
                    serde_json::json!([
                        {"type":"import_dna","dna":"start 17 998 store stop","position":[123,456]}
                    ]),
                );
                assert_eq!(status, DbStatus::Ok);
                assert_eq!(engine.population(), 3);
                let last = engine.snapshot().organisms.last().unwrap().id;
                assert_eq!(
                    engine
                        .dna(last)
                        .unwrap()
                        .blocks()
                        .iter()
                        .map(|block| (block.class as u8, block.value))
                        .collect::<Vec<_>>(),
                    vec![(9, 2), (0, 17), (0, 998), (7, 1), (9, 4), (10, 1)]
                );
                assert_default_source_actor(&engine, id, native);
                assert_eq!(
                    serde_json::to_value(&engine).unwrap()["species_templates"],
                    templates
                );
                assert!(
                    engine
                        .export_species_template_dna(SpeciesId::default())
                        .is_err()
                );
            }
        }
    }

    #[test]
    fn individual_default_imports_preserve_existing_startup_templates_with_or_without_raw_source() {
        for native in [true, false] {
            for retained_raw in [false, true] {
                let mut engine = source_fixture_engine(native);
                let original = "start 3 997 store stop\r\n'#tag:retained\r\n";
                let dna = engine.load_dna_source(original).unwrap();
                engine
                    .spawn_species_batch(dna, SpeciesId::default(), [[500.0; 2]; 2], 777)
                    .unwrap();
                if retained_raw {
                    engine
                        .retain_species_template_source(SpeciesId::default(), original.into())
                        .unwrap();
                }
                let templates = serde_json::to_value(&engine).unwrap()["species_templates"].clone();
                let exported = engine
                    .export_species_template_dna(SpeciesId::default())
                    .unwrap();
                for source in [original, "start 7 999 addstore stop\r\n"] {
                    assert_eq!(
                        db_engine_import_dna(&mut engine, source.as_ptr(), source.len()),
                        DbStatus::Ok
                    );
                    let (status, _) = classic_test_batch(
                        &mut engine,
                        serde_json::json!([
                            {"type":"import_dna","dna":source,"position":[321,654]}
                        ]),
                    );
                    assert_eq!(status, DbStatus::Ok);
                    let last = engine.snapshot().organisms.last().unwrap().id;
                    let literal = if source == original {
                        vec![(9, 2), (0, 3), (0, 997), (7, 1), (9, 4), (10, 1)]
                    } else {
                        vec![(9, 2), (0, 7), (0, 999), (7, 4), (9, 4), (10, 1)]
                    };
                    assert_eq!(
                        engine
                            .dna(last)
                            .unwrap()
                            .blocks()
                            .iter()
                            .map(|block| (block.class as u8, block.value))
                            .collect::<Vec<_>>(),
                        literal
                    );
                    assert_eq!(
                        serde_json::to_value(&engine).unwrap()["species_templates"],
                        templates
                    );
                    assert_eq!(
                        engine
                            .export_species_template_dna(SpeciesId::default())
                            .unwrap(),
                        exported
                    );
                }
                assert_eq!(engine.population(), 6);
                let resumed = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
                assert_eq!(
                    serde_json::to_value(&resumed).unwrap()["species_templates"],
                    templates
                );
                assert_eq!(
                    resumed
                        .export_species_template_dna(SpeciesId::default())
                        .unwrap(),
                    exported
                );
            }
        }
    }

    #[test]
    fn default_utf8_and_native_overflow_import_errors_preserve_entire_saved_state() {
        for native in [true, false] {
            let mut engine = source_fixture_engine(native);
            let before = SaveFile::encode(&engine).unwrap();
            let invalid_utf8 = [0xff];
            assert_eq!(
                db_engine_import_dna(&mut engine, invalid_utf8.as_ptr(), invalid_utf8.len()),
                DbStatus::InvalidUtf8
            );
            assert_eq!(SaveFile::encode(&engine).unwrap(), before);
            if native {
                let overflow = "start 32768 stop";
                assert_eq!(
                    db_engine_import_dna(&mut engine, overflow.as_ptr(), overflow.len()),
                    DbStatus::EngineError
                );
                assert_eq!(SaveFile::encode(&engine).unwrap(), before);
                let (status, result) = classic_test_batch(
                    &mut engine,
                    serde_json::json!([
                        {"type":"set_native_restart_seed","seed":999},
                        {"type":"import_dna","dna":overflow,"position":[500,500]}
                    ]),
                );
                assert_eq!(status, DbStatus::EngineError);
                assert!(result.is_empty());
                assert_eq!(SaveFile::encode(&engine).unwrap(), before);
            }
        }
    }
}
