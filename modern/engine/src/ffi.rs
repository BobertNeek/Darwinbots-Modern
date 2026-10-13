use crate::persistence::SaveFile;
use crate::{Engine, EngineConfig, LegacyDna, LegacyOrganismFile, OrganismId, SpeciesDefinition};
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
    ConfigureLocalMode {
        config: crate::LocalModeConfig,
    },
    UpdateCostSettings {
        costs: crate::DnaCosts,
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
        #[serde(default, rename = "initial_count")]
        _initial_count: usize,
        #[serde(default, rename = "placement")]
        _placement: Option<serde_json::Value>,
        #[serde(default)]
        skin: Option<[crate::SkinPoint; 4]>,
        #[serde(default)]
        restrictions: Option<crate::RestrictionColumn>,
        positions: Vec<[f32; 2]>,
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
    match Engine::new(config) {
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
    let dna = match LegacyDna::parse(text) {
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
            EngineCommand::Tick { count } => {
                if count > 1_000_000 {
                    return fail(DbStatus::InvalidCommand, "tick count exceeds batch limit");
                }
                engine.tick_many(count).map(|_| serde_json::Value::Null)
            }
            EngineCommand::ConfigureLocalMode { config } => engine
                .configure_local_mode(config)
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateCostSettings { costs } => engine
                .update_cost_settings(costs)
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
            EngineCommand::ImportDna { dna, position } => LegacyDna::parse(&dna).and_then(|dna| {
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
            }),
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
                _initial_count: _,
                _placement: _,
                skin,
                restrictions,
                positions,
            } => LegacyDna::parse(&dna).and_then(|dna| {
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
                let species = engine.register_species(definition);
                engine
                    .spawn_species_batch(dna, species, positions, initial_energy)
                    .map(|ids| {
                        serde_json::json!({
                            "species": species.0,
                            "count": ids.len(),
                            "compatibility_warnings": compatibility_warnings,
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
            } => LegacyDna::parse(&dna)
                .and_then(|dna| engine.replace_dna(OrganismId::new(slot, generation), dna))
                .map(|_| serde_json::Value::Null),
            EngineCommand::ExportDna { slot, generation } => engine
                .export_dna(OrganismId::new(slot, generation))
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
        Ok(loaded) => {
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
}
