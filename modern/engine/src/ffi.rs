use crate::persistence::SaveFile;
use crate::world::{NativePostDnaMemorySample, NativeProcessCaches};
use crate::{
    Engine, EngineConfig, EngineError, LegacyOrganismFile, NativeRedrawCadence, OrganismId,
    SpeciesDefinition,
};
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
struct NativeSetupAppearanceRequest {
    filename: Vec<u8>,
    dna: String,
    use_old_colors: bool,
    cold_uniform_state: Option<u32>,
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
struct RobotDragPlacementCommand {
    id: OrganismIdCommand,
    position: [f32; 2],
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
        #[serde(default)]
        capture_deaths: bool,
        #[serde(default)]
        native_redraw: NativeRedrawCadence,
        #[serde(default)]
        recording_weight: Option<u16>,
        #[serde(default)]
        capture_graph_samples: bool,
        #[serde(default)]
        require_native_repopulation_source: bool,
        #[serde(default)]
        cancelled_native_repopulation_lineages: Vec<u64>,
    },
    CaptureLivingGenomeRecords {
        weight_percent: u16,
    },
    SetOrdinaryRestart {
        enabled: bool,
    },
    FinishOrdinaryRestart,
    ReadNativeRestartSources,
    HasUnresolvedNativeRepopulationSources,
    BindNativeRestartSources {
        sources: Vec<crate::world::NativeRestartBinding>,
        #[serde(default)]
        preserve_existing: bool,
    },
    NativeRedraw {
        #[serde(default)]
        after_continuous_cycle: bool,
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
    ToggleNativeSun,
    ReadNativeProcessCaches,
    ReadRobotTagReport,
    SetNativeProcessCaches {
        caches: NativeProcessCaches,
    },
    ConfigureLocalMode {
        config: crate::LocalModeConfig,
    },
    ConfigureGraphSampling {
        settings: crate::GraphSamplingSettings,
    },
    ConfigureNativePostDnaMemoryWatch {
        request_id: u64,
        focus: Option<OrganismIdCommand>,
        capture_memory: Option<bool>,
    },
    ReadNativeGraphRecording,
    ReadNativeGraphSample,
    ReadNativeGraphResetSample,
    FeedNativeGraphSample {
        #[serde(default)]
        selective: bool,
        #[serde(default)]
        generational: bool,
        #[serde(default)]
        slow: bool,
        #[serde(default)]
        due_only: bool,
    },
    ReadNativeGraphLayout,
    UpdateNativeGraphLayout {
        #[serde(default)]
        graphs: Vec<crate::world::native_graph_layout::NativeGraphLayoutRow>,
        #[serde(default)]
        custom_queries: Vec<crate::world::native_graph_layout::NativeGraphCustomQuery>,
    },
    UpdateNativeGraphRecording {
        run_stamp: Option<String>,
        #[serde(default)]
        graphs: Vec<crate::world::native_graph_recording::NativeGraphRecordingRow>,
    },
    UpdateCostSettings {
        costs: Option<crate::DnaCosts>,
        preset: Option<u8>,
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
    ResumeRestartMode,
    ClearRestartModeHiding,
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
        #[serde(default)]
        suppress_death_poffs: bool,
        #[serde(default)]
        recording_weight: Option<u16>,
        #[serde(default)]
        exclude_repopulating: bool,
    },
    SetDeathPoffsEnabled {
        enabled: bool,
    },
    ImportSpecies {
        #[serde(default)]
        comment: String,
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
    UpdateSpeciesSettings {
        species: Vec<crate::SpeciesSettings>,
    },
    InsertSpecies {
        name: String,
        dna: String,
        position: [f32; 2],
        initial_energy: i32,
    },
    DragHighlightedRobots {
        anchor: OrganismIdCommand,
        grabbed: OrganismIdCommand,
        placements: Vec<RobotDragPlacementCommand>,
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
    TagOrganismsByName {
        founder_name: String,
        tag: String,
    },
    ReadRobotTag {
        slot: u32,
        generation: u32,
    },
    SetRobotTag {
        slot: u32,
        generation: u32,
        tag: String,
    },
    ExportDna {
        slot: u32,
        generation: u32,
    },
    ReadRobotDna {
        slot: u32,
        generation: u32,
        #[serde(default)]
        marker_position: Option<u32>,
    },
    ExportSpeciesTemplateDna {
        species: u32,
    },
    ListLegacyOrganismGroup {
        slot: u32,
        generation: u32,
    },
    ExportOrganismGroup {
        organisms: Vec<OrganismIdCommand>,
    },
    ReproduceNativeRobotInfo {
        slot: u32,
        generation: u32,
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
    AddTrashCompactor {
        left: crate::Obstacle,
        right: crate::Obstacle,
    },
    AddNativeRandomObstacles {
        count: usize,
        #[serde(default)]
        dimensions_permille: Option<[u16; 2]>,
    },
    AddNativeCenteredObstacle {
        dimensions_permille: [u16; 2],
    },
    AddNativePolarIce,
    AddNativeSimpleMaze {
        horizontal: bool,
        corridor: u16,
        thickness: u16,
    },
    AddNativePatternMaze {
        pattern: crate::world::NativePatternMaze,
        corridor: u16,
        thickness: u16,
    },
    SetShapeDrift {
        #[serde(default)]
        horizontal: Option<bool>,
        #[serde(default)]
        vertical: Option<bool>,
        #[serde(default)]
        rate: Option<i16>,
    },
    UpdateObstacle {
        obstacle: crate::Obstacle,
    },
    RemoveObstacle {
        id: u32,
    },
    RemoveNativeRandomObstacles,
    AddTeleporter {
        #[serde(flatten)]
        teleporter: crate::Teleporter,
    },
    AddNativeTeleporter {
        options: crate::NativeTeleporterOptions,
    },
    DragNativeTeleporter {
        id: u32,
        center: [f32; 2],
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
    SetShotDecay {
        energy: bool,
        do_not_decay: bool,
    },
    SetTiesEnabled {
        enabled: bool,
    },
    SetSpeciesColor {
        species: u32,
        color: u32,
    },
    SetOrganismColor {
        slot: u32,
        generation: u32,
        color: u32,
    },
    MakeNewSpecies {
        slot: u32,
        generation: u32,
        name: String,
        color: u32,
    },
    ForkRobotAndDescendants {
        slot: u32,
        generation: u32,
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
    SetPlayerBotKeyState {
        key: u32,
        pressed: bool,
    },
    SetPlayerBotTargets {
        focus: Option<OrganismIdCommand>,
        highlights: Vec<OrganismIdCommand>,
    },
    SetPlayerBotPointer {
        position: Option<[f32; 2]>,
    },
    SetOrganismMemory {
        slot: u32,
        generation: u32,
        address: u16,
        value: i32,
    },
    ResolveMemoryAddress {
        name: String,
        #[serde(default)]
        focus: Option<OrganismId>,
    },
    ExecuteRobots,
    FindBestRobot {
        #[serde(default)]
        energy_weighting: bool,
        #[serde(default)]
        weight_percent: Option<u16>,
    },
    SelectBestRobot {
        weight_percent: u16,
        #[serde(default)]
        restart_seed: Option<u64>,
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
        #[serde(default)]
        field_size: Option<u8>,
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

#[derive(serde::Serialize)]
struct LastError {
    message: String,
    #[serde(flatten)]
    details: Option<LastErrorDetails>,
}

#[derive(serde::Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
enum LastErrorDetails {
    MissingNativeRepopulationSource {
        lineage_id: u64,
        filename: String,
        directory: String,
    },
}

thread_local! {
    static LAST_ERROR: RefCell<LastError> = const { RefCell::new(LastError { message: String::new(), details: None }) };
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
    #[derive(serde::Serialize)]
    struct SnapshotWithDiagnostic<'a> {
        #[serde(flatten)]
        snapshot: &'a crate::Snapshot,
        native_post_dna_memory: &'a NativePostDnaMemorySample,
    }
    let snapshot = match engine.native_post_dna_memory_sample() {
        Some(sample) => serde_json::to_vec(&SnapshotWithDiagnostic {
            snapshot: engine.snapshot(),
            native_post_dna_memory: sample,
        }),
        None => serde_json::to_vec(engine.snapshot()),
    };
    match snapshot {
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

fn rebuild_archive_command_candidate(committed: &Engine) -> Result<Engine, EngineError> {
    let mut candidate = SaveFile::decode(&SaveFile::encode(committed)?)?;
    candidate.native_restarted_this_cycle = committed.native_restarted_this_cycle;
    candidate.player_bot_input = committed.player_bot_input.clone();
    candidate.inherit_native_post_dna_memory_watch(committed);
    Ok(candidate)
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
    // A prior tick could already have committed file transfers. A relinkable
    // failure must roll back the whole batch, including the optional seed.
    let interactive_repopulation = |command: &EngineCommand| {
        matches!(
            command,
            EngineCommand::Tick {
                require_native_repopulation_source,
                cancelled_native_repopulation_lineages,
                ..
            } if *require_native_repopulation_source || !cancelled_native_repopulation_lineages.is_empty()
        )
    };
    if batch.commands.iter().any(interactive_repopulation)
        && !matches!(
            batch.commands.as_slice(),
            [EngineCommand::Tick {
                count: 1,
                require_native_repopulation_source: true,
                ..
            }] | [
                EngineCommand::SetNativeRestartSeed { .. },
                EngineCommand::Tick {
                    count: 1,
                    require_native_repopulation_source: true,
                    ..
                }
            ]
        )
    {
        return fail(
            DbStatus::InvalidCommand,
            "requiring original repopulation DNA needs a single tick with an optional restart seed",
        );
    }
    if let [
        EngineCommand::ConfigureNativePostDnaMemoryWatch {
            request_id,
            focus,
            capture_memory,
        },
    ] = batch.commands.as_slice()
    {
        let response = serde_json::json!({
            "version": ABI_VERSION,
            "tick": engine.snapshot().tick,
            "population": engine.population(),
            "backend": format!("{:?}", engine.backend()),
            "results": [null],
        });
        return match serde_json::to_vec(&response) {
            Ok(bytes) => {
                engine.configure_native_post_dna_memory_watch(
                    *request_id,
                    focus
                        .as_ref()
                        .map(|id| OrganismId::new(id.slot, id.generation)),
                    capture_memory.unwrap_or(true),
                );
                write_buffer(out_buffer, bytes)
            }
            Err(error) => fail(DbStatus::EngineError, error),
        };
    }
    // Pure queries must not reconstruct the world or its GPU backend.
    if !batch.commands.is_empty()
        && batch.commands.iter().all(|command| {
            matches!(
                command,
                EngineCommand::ReadNativeGaussianSpare
                    | EngineCommand::ReadNativeSunChange
                    | EngineCommand::ReadNativeProcessCaches
                    | EngineCommand::ReadRobotTagReport
                    | EngineCommand::ReadNativeRestartSources
                    | EngineCommand::ReadNativeGraphRecording
                    | EngineCommand::ReadNativeGraphSample
                    | EngineCommand::ReadNativeGraphResetSample
                    | EngineCommand::FeedNativeGraphSample {
                        selective: false,
                        ..
                    }
                    | EngineCommand::ReadNativeGraphLayout
                    | EngineCommand::HasUnresolvedNativeRepopulationSources
                    | EngineCommand::ReadRobotTag { .. }
                    | EngineCommand::ReadRobotDna { .. }
                    | EngineCommand::ExportSpeciesTemplateDna { .. }
                    | EngineCommand::ResolveMemoryAddress { .. }
                    | EngineCommand::FindBestRobot { .. }
                    | EngineCommand::CaptureLivingGenomeRecords { .. }
                    | EngineCommand::ListLegacyOrganismGroup { .. }
            )
        })
    {
        if batch.commands.iter().any(|command| {
            matches!(command,
                EngineCommand::FindBestRobot { weight_percent: Some(weight), .. } if *weight > 200
            )
        }) {
            return fail(
                DbStatus::InvalidCommand,
                "Find Best weighting must be 0..200",
            );
        }
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
                EngineCommand::ReadNativeProcessCaches => engine
                    .read_native_process_caches()
                    .map(|caches| serde_json::json!(caches)),
                EngineCommand::ReadRobotTagReport => {
                    Ok(serde_json::json!(engine.robot_tag_report()))
                }
                EngineCommand::ReadNativeRestartSources => engine
                    .read_native_restart_sources()
                    .map(|sources| serde_json::json!(sources)),
                EngineCommand::ReadNativeGraphRecording => engine
                    .read_native_graph_recording()
                    .map(|state| serde_json::json!(state)),
                EngineCommand::ReadNativeGraphSample => {
                    Ok(serde_json::json!(engine.read_native_graph_sample()))
                }
                EngineCommand::ReadNativeGraphResetSample => {
                    Ok(serde_json::json!(engine.read_native_graph_reset_sample()))
                }
                EngineCommand::FeedNativeGraphSample {
                    selective: false,
                    generational,
                    slow,
                    due_only,
                } => Ok(serde_json::json!(engine.feed_native_graph_sample(
                    crate::GraphSamplingSettings {
                        selective: false,
                        generational: *generational,
                        slow: *slow,
                    },
                    *due_only,
                ))),
                EngineCommand::ReadNativeGraphLayout => engine
                    .read_native_graph_layout()
                    .map(|state| serde_json::json!(state)),
                EngineCommand::HasUnresolvedNativeRepopulationSources => engine
                    .has_unresolved_native_repopulation_sources()
                    .map(serde_json::Value::Bool),
                EngineCommand::ReadRobotTag { slot, generation } => engine
                    .read_robot_tag(OrganismId::new(*slot, *generation))
                    .map(serde_json::Value::String),
                EngineCommand::ReadRobotDna {
                    slot,
                    generation,
                    marker_position,
                } => engine
                    .read_robot_dna(OrganismId::new(*slot, *generation), *marker_position)
                    .map(|dna| serde_json::json!({ "dna": dna })),
                EngineCommand::ExportSpeciesTemplateDna { species } => engine
                    .export_species_template_dna(crate::SpeciesId(*species))
                    .map(|dna| serde_json::json!({ "dna": dna })),
                EngineCommand::CaptureLivingGenomeRecords { weight_percent } => engine
                    .living_genome_records(*weight_percent)
                    .map(|records| serde_json::json!(records)),
                EngineCommand::ResolveMemoryAddress { name, focus } => engine
                    .resolve_memory_address(name, *focus)
                    .map(|address| serde_json::json!({ "address": address })),
                EngineCommand::FindBestRobot {
                    energy_weighting,
                    weight_percent,
                } => {
                    let weight = weight_percent.unwrap_or(if *energy_weighting { 100 } else { 0 });
                    Ok(engine.find_best_robot_weighted(weight).map_or(
                        serde_json::Value::Null,
                        |id| serde_json::json!({"slot":id.slot(),"generation":id.generation()}),
                    ))
                }
                EngineCommand::ListLegacyOrganismGroup { slot, generation } => {
                    Ok(serde_json::json!(engine.legacy_organism_group(
                        OrganismId::new(*slot, *generation)
                    )))
                }
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
    let mut candidate = match committed_engine.fork_cpu_command_candidate() {
        Ok(Some(candidate)) => candidate,
        Ok(None) => match rebuild_archive_command_candidate(committed_engine) {
            Ok(candidate) => candidate,
            Err(error) => return fail(DbStatus::EngineError, error.to_string()),
        },
        Err(error) => return fail(DbStatus::EngineError, error),
    };
    let engine = &mut candidate;

    for command in batch.commands {
        let result = match command {
            EngineCommand::SetOrdinaryRestart { enabled } => engine
                .set_ordinary_restart(enabled)
                .map(|_| serde_json::Value::Null),
            EngineCommand::FinishOrdinaryRestart => engine
                .finish_ordinary_restart()
                .map(serde_json::Value::Bool),
            EngineCommand::ReadNativeRestartSources => engine
                .read_native_restart_sources()
                .map(|sources| serde_json::json!(sources)),
            EngineCommand::HasUnresolvedNativeRepopulationSources => engine
                .has_unresolved_native_repopulation_sources()
                .map(serde_json::Value::Bool),
            EngineCommand::BindNativeRestartSources {
                sources,
                preserve_existing,
            } => engine
                .bind_native_restart_sources(sources, preserve_existing)
                .map(|bound| serde_json::json!(bound)),
            EngineCommand::ConfigureNativePostDnaMemoryWatch {
                request_id,
                focus,
                capture_memory,
            } => {
                engine.configure_native_post_dna_memory_watch(
                    request_id,
                    focus.map(|id| OrganismId::new(id.slot, id.generation)),
                    capture_memory.unwrap_or(true),
                );
                Ok(serde_json::Value::Null)
            }
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
            EngineCommand::ToggleNativeSun => Ok(serde_json::json!(engine.toggle_native_sun())),
            EngineCommand::ReadNativeProcessCaches => engine
                .read_native_process_caches()
                .map(|caches| serde_json::json!(caches)),
            EngineCommand::ReadRobotTagReport => Ok(serde_json::json!(engine.robot_tag_report())),
            EngineCommand::SetNativeProcessCaches { caches } => engine
                .set_native_process_caches(caches)
                .map(|_| serde_json::Value::Null),
            EngineCommand::CaptureLivingGenomeRecords { weight_percent } => engine
                .living_genome_records(weight_percent)
                .map(|records| serde_json::json!(records)),
            EngineCommand::Tick {
                count,
                capture_deaths,
                recording_weight,
                capture_graph_samples,
                require_native_repopulation_source,
                cancelled_native_repopulation_lineages,
                native_redraw,
            } => {
                if count > 1_000_000 {
                    return fail(DbStatus::InvalidCommand, "tick count exceeds batch limit");
                }
                let cancelled_lineages = require_native_repopulation_source
                    .then_some(cancelled_native_repopulation_lineages.as_slice());
                let mut samples = Vec::new();
                let capture = capture_graph_samples.then_some(&mut samples);
                if capture_deaths {
                    engine.tick_many_recording_deaths(
                        count, cancelled_lineages, capture, recording_weight.unwrap_or(100), native_redraw,
                    ).map(|records| {
                        if capture_graph_samples {
                            serde_json::json!({"death_records": records, "graph_samples": samples})
                        } else {
                            serde_json::json!({"death_records": records})
                        }
                    })
                } else if capture_graph_samples {
                    engine
                        .tick_many_with_native_options(
                            count,
                            cancelled_lineages,
                            capture,
                            native_redraw,
                        )
                        .map(|_| serde_json::json!({"graph_samples": samples}))
                } else {
                    engine
                        .tick_many_with_native_options(
                            count,
                            cancelled_lineages,
                            None,
                            native_redraw,
                        )
                        .map(|_| serde_json::Value::Null)
                }
            }
            EngineCommand::NativeRedraw {
                after_continuous_cycle,
            } => {
                if !after_continuous_cycle || !engine.native_restarted_this_cycle {
                    engine.native_redraw();
                }
                Ok(serde_json::Value::Null)
            }
            EngineCommand::ConfigureLocalMode { config } => engine
                .configure_local_mode(config)
                .map(|_| serde_json::Value::Null),
            EngineCommand::ConfigureGraphSampling { settings } => {
                engine.configure_graph_sampling(settings);
                Ok(serde_json::Value::Null)
            }
            EngineCommand::ReadNativeGraphRecording => engine
                .read_native_graph_recording()
                .map(|state| serde_json::json!(state)),
            EngineCommand::ReadNativeGraphSample => {
                Ok(serde_json::json!(engine.read_native_graph_sample()))
            }
            EngineCommand::ReadNativeGraphResetSample => {
                Ok(serde_json::json!(engine.read_native_graph_reset_sample()))
            }
            EngineCommand::FeedNativeGraphSample {
                selective,
                generational,
                slow,
                due_only,
            } => Ok(serde_json::json!(engine.feed_native_graph_sample(
                crate::GraphSamplingSettings {
                    selective,
                    generational,
                    slow
                },
                due_only
            ))),
            EngineCommand::ReadNativeGraphLayout => engine
                .read_native_graph_layout()
                .map(|state| serde_json::json!(state)),
            EngineCommand::UpdateNativeGraphLayout {
                graphs,
                custom_queries,
            } => engine
                .update_native_graph_layout(graphs, custom_queries)
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateNativeGraphRecording { run_stamp, graphs } => engine
                .update_native_graph_recording(run_stamp, graphs)
                .map(|_| serde_json::Value::Null),
            EngineCommand::UpdateCostSettings { costs, preset } => engine
                .update_cost_settings_with_preset(costs, preset)
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
            EngineCommand::ResumeRestartMode => engine
                .resume_restart_mode()
                .map(|_| serde_json::Value::Null),
            EngineCommand::ClearRestartModeHiding => {
                engine.clear_restart_mode_hiding();
                Ok(serde_json::Value::Null)
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
            EngineCommand::SetDeathPoffsEnabled { enabled } => {
                engine.set_death_poffs_enabled(enabled);
                Ok(serde_json::Value::Null)
            }
            EngineCommand::RemoveMany {
                organisms,
                suppress_death_poffs,
                recording_weight,
                exclude_repopulating,
            } => {
                let ids: Vec<_> = organisms
                    .into_iter()
                    .map(|organism| OrganismId::new(organism.slot, organism.generation))
                    .collect();
                if let Some(weight) = recording_weight {
                    engine
                        .remove_many_recording_deaths(
                            &ids,
                            suppress_death_poffs,
                            weight,
                            exclude_repopulating,
                        )
                        .map(|records| serde_json::json!(records))
                } else {
                    engine
                        .remove_many_with_death_poffs(&ids, suppress_death_poffs)
                        .map(|_| serde_json::Value::Null)
                }
            }
            EngineCommand::ImportSpecies {
                comment,
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
                        engine.retain_species_template_comment(species, comment);
                        Ok(serde_json::json!({
                            "species": species.0,
                            "count": ids.len(),
                            "compatibility_warnings": compatibility_warnings,
                        }))
                    })
                })
            }
            EngineCommand::UpdateSpeciesSettings { species } => engine
                .update_species_settings(species)
                .map(|()| serde_json::json!({ "updated": true })),
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
            EngineCommand::DragHighlightedRobots {
                anchor,
                grabbed,
                placements,
            } => engine
                .drag_highlighted_robots(
                    OrganismId::new(anchor.slot, anchor.generation),
                    OrganismId::new(grabbed.slot, grabbed.generation),
                    &placements
                        .into_iter()
                        .map(|placement| {
                            (
                                OrganismId::new(placement.id.slot, placement.id.generation),
                                placement.position,
                            )
                        })
                        .collect::<Vec<_>>(),
                )
                .map(|_| serde_json::Value::Null),
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
            EngineCommand::TagOrganismsByName { founder_name, tag } => engine
                .tag_organisms_by_name(&founder_name, &tag)
                .map(|count| serde_json::json!({ "count": count })),
            EngineCommand::ReadRobotTag { slot, generation } => engine
                .read_robot_tag(OrganismId::new(slot, generation))
                .map(serde_json::Value::String),
            EngineCommand::SetRobotTag {
                slot,
                generation,
                tag,
            } => engine
                .set_robot_tag(OrganismId::new(slot, generation), &tag)
                .map(serde_json::Value::String),
            EngineCommand::ExportDna { slot, generation } => engine
                .export_dna(OrganismId::new(slot, generation))
                .map(|dna| serde_json::json!({ "dna": dna })),
            EngineCommand::ReadRobotDna {
                slot,
                generation,
                marker_position,
            } => engine
                .read_robot_dna(OrganismId::new(slot, generation), marker_position)
                .map(|dna| serde_json::json!({ "dna": dna })),
            EngineCommand::ExportSpeciesTemplateDna { species } => engine
                .export_species_template_dna(crate::SpeciesId(species))
                .map(|dna| serde_json::json!({ "dna": dna })),
            EngineCommand::ListLegacyOrganismGroup { slot, generation } => Ok(serde_json::json!(
                engine.legacy_organism_group(OrganismId::new(slot, generation))
            )),
            EngineCommand::ExportOrganismGroup { organisms } => {
                let members: Vec<_> = organisms
                    .into_iter()
                    .map(|id| OrganismId::new(id.slot, id.generation))
                    .collect();
                engine
                    .export_original_organism_file(&members)
                    .map(|bytes| serde_json::json!({ "payload": bytes }))
            }
            EngineCommand::ReproduceNativeRobotInfo { slot, generation } => engine
                .reproduce_native_robot_info(OrganismId::new(slot, generation))
                .map(|child| {
                    child.map_or(serde_json::Value::Null, |id|
                    serde_json::json!({ "slot": id.slot(), "generation": id.generation() }))
                }),
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
            EngineCommand::AddTrashCompactor { left, right } => engine
                .add_trash_compactor(left, right)
                .map(|_| serde_json::Value::Null),
            EngineCommand::AddNativeRandomObstacles {
                count,
                dimensions_permille,
            } => engine
                .add_native_random_obstacles(
                    count,
                    dimensions_permille.unwrap_or(crate::world::DEFAULT_NATIVE_SHAPE_DIMENSIONS),
                )
                .map(|added| serde_json::json!(added)),
            EngineCommand::AddNativeCenteredObstacle {
                dimensions_permille,
            } => engine
                .add_native_centered_obstacle(dimensions_permille)
                .map(|added| serde_json::json!(added)),
            EngineCommand::AddNativePolarIce => engine.add_native_polar_ice().map(|result| {
                result.map_or(
                    serde_json::Value::Null,
                    |(added, drift)| serde_json::json!({ "added": added, "drift": drift }),
                )
            }),
            EngineCommand::AddNativeSimpleMaze {
                horizontal,
                corridor,
                thickness,
            } => engine
                .add_native_simple_maze(horizontal, corridor, thickness)
                .map(|added| serde_json::json!(added)),
            EngineCommand::AddNativePatternMaze {
                pattern,
                corridor,
                thickness,
            } => engine
                .add_native_pattern_maze(pattern, corridor, thickness)
                .map(|added| serde_json::json!(added)),
            EngineCommand::SetShapeDrift {
                horizontal,
                vertical,
                rate,
            } => engine
                .set_shape_drift(horizontal, vertical, rate)
                .map(|settings| serde_json::json!(settings)),
            EngineCommand::UpdateObstacle { obstacle } => engine
                .update_obstacle(obstacle)
                .map(|_| serde_json::Value::Null),
            EngineCommand::RemoveObstacle { id } => {
                engine.remove_obstacle(id).map(|_| serde_json::Value::Null)
            }
            EngineCommand::RemoveNativeRandomObstacles => engine
                .remove_native_random_obstacles()
                .map(|removed| serde_json::json!(removed)),
            EngineCommand::AddTeleporter { teleporter } => engine
                .add_teleporter(teleporter)
                .map(|_| serde_json::Value::Null),
            EngineCommand::AddNativeTeleporter { options } => engine
                .add_native_teleporter(options)
                .map(|id| serde_json::json!({ "id": id })),
            EngineCommand::DragNativeTeleporter { id, center } => engine
                .drag_native_teleporter(id, center)
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
            EngineCommand::SetShotDecay {
                energy,
                do_not_decay,
            } => {
                engine.set_shot_decay(energy, do_not_decay);
                Ok(serde_json::Value::Null)
            }
            EngineCommand::SetTiesEnabled { enabled } => {
                engine.set_ties_enabled(enabled);
                Ok(serde_json::Value::Null)
            }
            EngineCommand::SetSpeciesColor { species, color } => engine
                .set_species_color(crate::SpeciesId(species), color)
                .map(|_| serde_json::Value::Null),
            EngineCommand::SetOrganismColor {
                slot,
                generation,
                color,
            } => engine
                .set_organism_color(OrganismId::new(slot, generation), color)
                .map(|_| serde_json::Value::Null),
            EngineCommand::MakeNewSpecies {
                slot,
                generation,
                name,
                color,
            } => engine
                .make_new_species(OrganismId::new(slot, generation), name, color)
                .map(|species| serde_json::json!({ "species": species.0 })),
            EngineCommand::ForkRobotAndDescendants { slot, generation } => engine
                .fork_robot_and_descendants(OrganismId::new(slot, generation))
                .map(|name| serde_json::json!({ "name": name })),
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
            EngineCommand::SetPlayerBotKeyState { key, pressed } => engine
                .set_player_bot_key_state(key, pressed)
                .map(|_| serde_json::Value::Null),
            EngineCommand::SetPlayerBotTargets { focus, highlights } => {
                engine.set_player_bot_targets(
                    focus.map(|value| OrganismId::new(value.slot, value.generation)),
                    highlights
                        .into_iter()
                        .map(|value| OrganismId::new(value.slot, value.generation))
                        .collect(),
                );
                Ok(serde_json::Value::Null)
            }
            EngineCommand::SetPlayerBotPointer { position } => engine
                .set_player_bot_pointer(position)
                .map(|_| serde_json::Value::Null),
            EngineCommand::SetOrganismMemory {
                slot,
                generation,
                address,
                value,
            } => engine
                .set_organism_memory(OrganismId::new(slot, generation), address, value)
                .map(|_| serde_json::Value::Null),
            EngineCommand::ResolveMemoryAddress { name, focus } => engine
                .resolve_memory_address(&name, focus)
                .map(|address| serde_json::json!({ "address": address })),
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
            EngineCommand::SelectBestRobot {
                weight_percent,
                restart_seed,
            } => {
                if weight_percent > 200 {
                    return fail(
                        DbStatus::InvalidCommand,
                        "Find Best weighting must be 0..200",
                    );
                }
                engine.select_best_robot(weight_percent, restart_seed).map(|(selected, restarted, weight)| {
                    serde_json::json!({
                        "selected": selected.map(|id| serde_json::json!({"slot":id.slot(),"generation":id.generation()})),
                        "restarted": restarted,
                        "effective_weight": weight,
                    })
                })
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
                field_size,
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
                .and_then(|_| match field_size {
                    Some(preset) => engine.set_live_field_size(preset),
                    None => Ok(()),
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
            Err(error) => return fail_engine(error),
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
pub extern "C" fn db_engine_save_without_restart_mode(
    engine: *const Engine,
    out_buffer: *mut DbBuffer,
) -> DbStatus {
    save_legacy(engine, out_buffer, SaveFile::encode_without_restart_mode)
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_save_legacy(
    engine: *const Engine,
    out_buffer: *mut DbBuffer,
) -> DbStatus {
    save_legacy(engine, out_buffer, Engine::export_native_sim)
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_save_legacy_without_mutation_details(
    engine: *const Engine,
    out_buffer: *mut DbBuffer,
) -> DbStatus {
    save_legacy(
        engine,
        out_buffer,
        Engine::export_native_sim_without_mutation_details,
    )
}

fn save_legacy(
    engine: *const Engine,
    out_buffer: *mut DbBuffer,
    export: fn(&Engine) -> Result<Vec<u8>, crate::EngineError>,
) -> DbStatus {
    let (Some(engine), Some(out_buffer)) =
        (unsafe { engine.as_ref() }, unsafe { out_buffer.as_mut() })
    else {
        return DbStatus::NullPointer;
    };
    if !out_buffer.data.is_null() {
        return fail(DbStatus::EngineError, "output buffer must be empty");
    }
    match export(engine) {
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
        db_engine_load_impl(engine, save_data, save_len, 0)
    })
}

/// Read the native uniform stream without consuming randomness. Minus one
/// denotes an absent engine or a Portable world.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_native_setup_uniform_state(engine: *const Engine) -> i32 {
    unsafe { engine.as_ref() }
        .and_then(Engine::native_setup_uniform_state)
        .map_or(-1, |state| state as i32)
}

/// Stage the original appearance using the caller's local Single Timer at
/// the two source call sites. A null world means pre-start Setup. A Portable
/// world or DNA that has no native LoadDNA image yields JSON null.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_stage_native_species_appearance(
    engine: *mut Engine,
    request: *const u8,
    request_len: usize,
    sample_timer: Option<extern "C" fn() -> f32>,
    out: *mut DbBuffer,
) -> DbStatus {
    ffi_guard("stage native species appearance", || {
        let (Some(sample_timer), Some(out)) = (sample_timer, unsafe { out.as_mut() }) else {
            return DbStatus::NullPointer;
        };
        if request.is_null() || request_len == 0 {
            return fail(DbStatus::InvalidCommand, "appearance request is empty");
        }
        if !out.data.is_null() {
            return fail(DbStatus::EngineError, "output buffer must be empty");
        }
        let bytes = unsafe { slice::from_raw_parts(request, request_len) };
        let request: NativeSetupAppearanceRequest = match serde_json::from_slice(bytes) {
            Ok(request) => request,
            Err(error) => return fail(DbStatus::InvalidCommand, error),
        };
        let mut cold_random;
        let random = if let Some(engine) = unsafe { engine.as_mut() } {
            match engine.native_setup_random() {
                Some(random) => random,
                None => return write_buffer(out, b"null".to_vec()),
            }
        } else {
            cold_random = match request.cold_uniform_state {
                Some(state) => {
                    match crate::native_random::NativeRandom::from_uniform_state(state) {
                        Some(random) => random,
                        None => {
                            return fail(DbStatus::InvalidCommand, "invalid native uniform state");
                        }
                    }
                }
                None => crate::native_random::NativeRandom::default(),
            };
            &mut cold_random
        };
        let result = crate::native_skin::assign_staged_species_appearance(
            random,
            &request.filename,
            &request.dna,
            request.use_old_colors,
            || sample_timer(),
        );
        match result {
            Ok(appearance) => write_buffer(
                out,
                serde_json::json!({
                    "argb": appearance.argb,
                    "skin": appearance.skin,
                    "uniform_state": random.state(),
                })
                .to_string()
                .into_bytes(),
            ),
            Err(EngineError::DnaParse { .. }) => write_buffer(out, b"null".to_vec()),
            Err(error) => fail(DbStatus::EngineError, error),
        }
    })
}

/// A nonzero VB6 Long overrides the seed of a native simulation load.
/// Zero keeps the archive seed. Modern checkpoints always keep their RNG.
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_load_with_native_seed(
    engine: *mut Engine,
    save_data: *const u8,
    save_len: usize,
    seed: i32,
) -> DbStatus {
    ffi_guard("load engine", || {
        db_engine_load_impl(engine, save_data, save_len, seed)
    })
}

/// Set the live native seed field without restarting its random stream.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[unsafe(no_mangle)]
pub extern "C" fn db_engine_set_native_seed_metadata(engine: *mut Engine, seed: i32) -> DbStatus {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return DbStatus::NullPointer;
    };
    engine.set_native_seed_metadata(seed);
    DbStatus::Ok
}

fn db_engine_load_impl(
    engine: *mut Engine,
    save_data: *const u8,
    save_len: usize,
    native_seed: i32,
) -> DbStatus {
    let Some(engine) = (unsafe { engine.as_mut() }) else {
        return DbStatus::NullPointer;
    };
    if save_data.is_null() || save_len == 0 {
        return fail(DbStatus::EngineError, "save buffer is empty");
    }
    let bytes = unsafe { slice::from_raw_parts(save_data, save_len) };
    match SaveFile::decode_with_native_seed(bytes, native_seed) {
        Ok(mut loaded) => {
            // Native .sim omits Global.gset preferences. DB3S instead restores
            // its complete saved policy; the two load formats stay distinct.
            if !bytes.starts_with(b"DB3S") {
                loaded.retain_native_global_preferences(engine.configuration());
                loaded.inherit_native_gaussian_spare(engine);
                loaded.inherit_native_sun_indicator_after_sim_load(engine);
                if let Err(error) = loaded.inherit_native_process_caches_after_sim_load(engine) {
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
    #[derive(serde::Serialize)]
    struct ErrorResponse<'a> {
        version: u32,
        #[serde(flatten)]
        error: &'a LastError,
    }
    let bytes = LAST_ERROR.with(|error| {
        serde_json::to_vec(&ErrorResponse {
            version: ABI_VERSION,
            error: &error.borrow(),
        })
    });
    match bytes {
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
    LAST_ERROR.with(|last_error| {
        *last_error.borrow_mut() = LastError {
            message: error.to_string(),
            details: None,
        };
    });
    status
}

fn fail_engine(error: EngineError) -> DbStatus {
    let message = error.to_string();
    let details = match error {
        EngineError::MissingNativeRepopulationSource {
            lineage_id,
            filename,
            directory,
        } => Some(LastErrorDetails::MissingNativeRepopulationSource {
            lineage_id,
            filename,
            directory,
        }),
        _ => None,
    };
    LAST_ERROR.with(|last_error| *last_error.borrow_mut() = LastError { message, details });
    DbStatus::EngineError
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
            "repopulation_counter": 0,
            "energy": {
                "ring": ring,
                "current_index": current_index,
                "displayed": displayed,
            }
        }))
        .unwrap()
    }

    #[test]
    fn native_menu_seed_metadata_changes_the_archive_seed_without_touching_rng_or_portable_worlds()
    {
        use crate::native_binary::{NativeRecordFile, NativeSimulationTail};

        let archive = include_bytes!("../tests/fixtures/db24832-movement-start-48032.sim");
        let mut native = Engine::from_native_sim(archive).unwrap();
        native.set_native_gaussian_spare(Some(0.125)).unwrap();
        let mut expected = native.db3s_v3_payload().unwrap();
        assert_eq!(expected["config"]["seed"], 48_032);

        for seed in [-1, 0] {
            assert_eq!(
                db_engine_set_native_seed_metadata(&mut native, seed),
                DbStatus::Ok
            );
            expected["config"]["seed"] = serde_json::json!(u64::from(seed as u32));
            assert_eq!(native.db3s_v3_payload().unwrap(), expected);
            let exported = NativeRecordFile::decode(&native.export_native_sim().unwrap()).unwrap();
            let tail = NativeSimulationTail::decode(&exported.tail).unwrap();
            assert_eq!(tail.integer("seed").unwrap(), seed);
        }

        let mut portable = Engine::new(EngineConfig::testing()).unwrap();
        let before = SaveFile::encode(&portable).unwrap();
        assert_eq!(
            db_engine_set_native_seed_metadata(&mut portable, 3),
            DbStatus::Ok
        );
        assert_eq!(SaveFile::encode(&portable).unwrap(), before);
    }

    #[test]
    fn native_menu_load_seed_controls_only_the_installed_simulation() {
        use crate::native_binary::{NativeRecordFile, NativeSimulationTail};

        let previous_file = include_bytes!("../tests/fixtures/db24832-original-user.sim");
        let selected_file = include_bytes!("../tests/fixtures/db24832-movement-start-48032.sim");
        let mut current = Engine::from_native_sim(previous_file).unwrap();
        assert_eq!(current.configuration().seed, 6_341_122);
        current.set_native_gaussian_spare(Some(0.125)).unwrap();
        let before = SaveFile::encode(&current).unwrap();
        let invalid = [0];
        assert_eq!(
            db_engine_load_with_native_seed(&mut current, invalid.as_ptr(), invalid.len(), 1_234),
            DbStatus::EngineError
        );
        assert_eq!(SaveFile::encode(&current).unwrap(), before);

        for (candidate, seed, uniform) in
            [(6_341_122, 6_341_122, 6_414_899), (0, 48_032, 4_535_603)]
        {
            assert_eq!(
                db_engine_load_with_native_seed(
                    &mut current,
                    selected_file.as_ptr(),
                    selected_file.len(),
                    candidate,
                ),
                DbStatus::Ok
            );
            assert_eq!(current.configuration().seed, seed);
            assert_eq!(current.snapshot().native_cycle, Some(0));
            let installed = current.db3s_v3_payload().unwrap();
            let native_random = &installed["native_random"];
            assert_eq!(native_random["state"], uniform);
            assert_eq!(native_random["gaussian_spare"], 0.125);
            let exported = NativeRecordFile::decode(&current.export_native_sim().unwrap()).unwrap();
            let tail = NativeSimulationTail::decode(&exported.tail).unwrap();
            assert_eq!(tail.integer("seed").unwrap(), seed as i32);
        }

        assert_eq!(
            db_engine_load_with_native_seed(&mut current, before.as_ptr(), before.len(), 1_234),
            DbStatus::Ok
        );
        assert_eq!(SaveFile::encode(&current).unwrap(), before);
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
            serde_json::to_value(fresh_process.read_native_process_caches().unwrap()).unwrap();
        assert_eq!(fresh_caches["shots_this_cycle"], 0);
        assert_eq!(saved.configuration().vegetation.repopulation_cooldown, 10);
        assert_eq!(fresh_caches["repopulation_counter"], -10);
        assert_eq!(fresh_caches["death_poffs_enabled"], true);
        assert!(fresh_process.snapshot().death_poffs_enabled);
        assert_eq!(fresh_caches["energy"]["displayed"], 0);
        assert_eq!(fresh_caches["energy"]["current_index"], 0);
        assert_eq!(
            fresh_caches["energy"]["ring"],
            serde_json::json!(vec![0_i32; 101])
        );

        let mut current = Engine::new_native(EngineConfig::testing()).unwrap();
        let mut current_caches = native_cache_fixture(0.25, 109, 7, 37, 17);
        current_caches.death_poffs_enabled = false;
        current_caches.repopulation_counter = Some(17);
        let mut current_context = serde_json::to_value(&current_caches).unwrap();
        current.set_native_process_caches(current_caches).unwrap();
        assert_eq!(
            db_engine_load(&mut current, original_bytes.as_ptr(), original_bytes.len()),
            DbStatus::Ok
        );
        assert_eq!(current.native_light_availability(), 0.25);
        // startloaded resets only this process counter to the newly loaded delay.
        current_context["repopulation_counter"] = serde_json::json!(-10);
        assert_eq!(
            serde_json::to_value(current.read_native_process_caches().unwrap()).unwrap(),
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
            serde_json::to_value(current.read_native_process_caches().unwrap()).unwrap(),
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
        current.set_death_poffs_enabled(false);
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
        let mut caches = native_cache_fixture(0.375, 421, 29, 199, 17);
        caches.death_poffs_enabled = false;
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
        let before_policy = serde_json::to_value(&engine).unwrap();
        let publications = engine.snapshot_publication_count();
        let (status, response) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"set_death_poffs_enabled","enabled":false},
                {"type":"read_native_process_caches"}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert!(response["results"][0].is_null());
        assert_eq!(response["results"][1]["death_poffs_enabled"], false);
        assert!(!engine.snapshot().death_poffs_enabled);
        let after_policy = serde_json::to_value(&engine).unwrap();
        for key in ["native_random", "slots", "config", "tick", "native_cycle"] {
            assert_eq!(after_policy[key], before_policy[key], "{key}");
        }
        assert_eq!(engine.snapshot_publication_count(), publications);
        assert_eq!(after_policy["death_poffs_enabled"], false);
        let resumed = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        assert!(!resumed.snapshot().death_poffs_enabled);
        assert!(
            !resumed
                .read_native_process_caches()
                .unwrap()
                .death_poffs_enabled
        );
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
        for flag in [
            serde_json::Value::Null,
            serde_json::json!(0),
            serde_json::json!("false"),
        ] {
            let mut invalid = valid.clone();
            invalid["death_poffs_enabled"] = flag;
            invalid_cases.push(invalid);
        }
        for invalid in invalid_cases {
            let before = SaveFile::encode(&engine).unwrap();
            let (status, response) = classic_test_batch(
                &mut engine,
                serde_json::json!([
                    {"type":"set_organism_energy","slot":id.slot(),"generation":id.generation(),"energy":42},
                    {"type":"set_native_process_caches","caches":valid},
                    {"type":"set_death_poffs_enabled","enabled":false},
                    {"type":"set_native_process_caches","caches":invalid}
                ]),
            );
            assert_ne!(status, DbStatus::Ok);
            assert!(response.is_empty());
            assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        }
    }

    #[test]
    fn death_particle_command_batches_keep_late_failures_atomic_and_explicit_suppression_silent() {
        let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
        let first = engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0, 500.0])
            .unwrap();
        let second = engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [700.0, 500.0])
            .unwrap();
        let mut state = engine.db3s_v3_payload().unwrap();
        state["slots"][second.slot() as usize]["organism"]["kinematics"]["velocity"] =
            serde_json::json!([40000.0, 0.0]);
        engine = Engine::restore(Engine::from_db3s_v3_payload(state).unwrap()).unwrap();
        let before = SaveFile::encode(&engine).unwrap();
        let snapshot = engine.snapshot().clone();
        let publications = engine.snapshot_publication_count();
        let backend = engine.backend();
        let ids = serde_json::json!([
            {"slot":first.slot(),"generation":first.generation()},
            {"slot":second.slot(),"generation":second.generation()}
        ]);
        let (status, response) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"set_organism_energy","slot":first.slot(),"generation":first.generation(),"energy":42},
                {"type":"set_death_poffs_enabled","enabled":true},
                {"type":"remove_many","organisms":ids}
            ]),
        );
        assert_eq!(status, DbStatus::EngineError);
        assert!(response.is_empty());
        assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        assert_eq!(engine.snapshot(), &snapshot);
        assert_eq!(engine.snapshot_publication_count(), publications);
        assert_eq!(engine.backend(), backend);
        for command in [
            serde_json::json!({"type":"set_death_poffs_enabled"}),
            serde_json::json!({"type":"set_death_poffs_enabled","enabled":0}),
            serde_json::json!({"type":"remove_many","organisms":ids,"suppress_death_poffs":"true"}),
        ] {
            let (status, response) = classic_test_batch(
                &mut engine,
                serde_json::json!([
                    {"type":"set_death_poffs_enabled","enabled":false}, command
                ]),
            );
            assert_eq!(status, DbStatus::InvalidCommand);
            assert!(response.is_empty());
            assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        }
        let random = serde_json::to_value(&engine).unwrap()["native_random"].clone();
        let (status, _) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"remove_many","organisms":ids,"suppress_death_poffs":true}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        assert_eq!(engine.population(), 0);
        assert!(engine.snapshot().shots.is_empty());
        assert!(engine.snapshot().death_poffs_enabled);
        assert_eq!(
            serde_json::to_value(&engine).unwrap()["native_random"],
            random
        );
    }

    #[test]
    fn manual_best_selection_keeps_legacy_queries_pure_and_rejects_batches_atomically() {
        let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
        let id = engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0, 500.0])
            .unwrap();
        let before = SaveFile::encode(&engine).unwrap();
        let (status, response) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"find_best_robot"},
                {"type":"find_best_robot","energy_weighting":true},
                {"type":"find_best_robot","weight_percent":137}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
        let key = serde_json::json!({"slot":id.slot(),"generation":id.generation()});
        for index in 0..3 {
            assert_eq!(response["results"][index], key);
        }
        assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        let (status, response) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"select_best_robot","weight_percent":137}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(
            response["results"][0],
            serde_json::json!({"selected":key,"restarted":false,"effective_weight":137})
        );
        assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        let (status, response) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"find_best_robot","weight_percent":201}
            ]),
        );
        assert_eq!(status, DbStatus::InvalidCommand);
        assert!(response.is_empty());
        assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        for bad in [
            serde_json::json!({"type":"select_best_robot","weight_percent":201}),
            serde_json::json!({"type":"set_organism_energy","slot":id.slot(),"generation":id.generation()+1,"energy":42}),
        ] {
            let (status, response) = classic_test_batch(
                &mut engine,
                serde_json::json!([
                    {"type":"set_organism_energy","slot":id.slot(),"generation":id.generation(),"energy":42},
                    {"type":"select_best_robot","weight_percent":100},
                    bad
                ]),
            );
            assert_ne!(status, DbStatus::Ok);
            assert!(response.is_empty());
            assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        }
        let other = engine
            .spawn_at(LegacyDna::parse("stop").unwrap(), [700.0, 500.0])
            .unwrap();
        assert_eq!(engine.find_best_robot_weighted(200), Some(other));
        let (status, response) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"set_organism_energy","slot":id.slot(),"generation":id.generation(),"energy":2000},
                {"type":"find_best_robot","weight_percent":200},
                {"type":"select_best_robot","weight_percent":200}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(response["results"][1], key);
        assert_eq!(response["results"][2]["selected"], key);
        assert_eq!(engine.organism(id).unwrap().energy, 2000);
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
        let message = LAST_ERROR.with(|error| error.borrow().message.clone());
        assert!(message.contains("test operation panicked"));
        assert!(message.contains("deliberate failure"));
    }
    #[test]
    fn native_robot_dna_reads_raw_text_with_a_one_based_block_marker() {
        use serde_json::json;

        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("def FirstAlias 1\ndef LastAlias 1\nstart 123 .up store stop")
            .unwrap();
        let snapshot_source = dna.to_snapshot_source();
        let id = world.spawn(dna).unwrap();
        let mut read = json!({"type": "read_robot_dna", "slot": id.slot(),
                             "generation": id.generation()});
        let plain = concat!(
            "\r\n start\r\n 123 .LastAlias store\r\n stop\r\n",
            "''''''''''''''''''''''''  Gene:  1 Ends at position  5  '''''''''''''''''''''''\r\n",
        );
        let marker = " '[<POSITION MARKER]\r\n";
        let saved = SaveFile::encode(&world).unwrap();
        let publications = world.snapshot_publication_count();
        let exported = world.export_dna(id).unwrap();
        assert_eq!(snapshot_source, plain.replace(".LastAlias", ".up"));

        for (position, expected) in [
            (None, plain.to_owned()),
            (Some(0), plain.to_owned()),
            (
                Some(1),
                plain.replace(" start\r\n", &format!(" start\r\n{marker}")),
            ),
            (Some(2), plain.replace(" 123", &format!(" 123{marker}"))),
            (Some(5), format!("{plain}{marker}")),
            (Some(6), plain.to_owned()), // Master end is never displayed.
            (Some(u32::MAX), plain.to_owned()),
        ] {
            if let Some(position) = position {
                read["marker_position"] = json!(position);
            }
            let (status, result) = classic_test_batch(&mut world, json!([read]));
            assert_eq!(status, DbStatus::Ok);
            let result: serde_json::Value = serde_json::from_slice(&result).unwrap();
            assert_eq!(result["results"][0], json!({"dna": expected}));
        }
        read["marker_position"] = json!(null);
        let (status, unmarked) = classic_test_batch(&mut world, json!([read]));
        assert_eq!(status, DbStatus::Ok);
        let result: serde_json::Value = serde_json::from_slice(&unmarked).unwrap();
        assert_eq!(result["results"][0], json!({"dna": plain}));
        read["generation"] = json!(id.generation() + 1);
        let (status, result) = classic_test_batch(&mut world, json!([read]));
        assert_eq!(status, DbStatus::EngineError);
        assert!(result.is_empty());
        assert_eq!(world.export_dna(id).unwrap(), exported);
        assert_eq!(SaveFile::encode(&world).unwrap(), saved);
        assert_eq!(world.snapshot_publication_count(), publications);
    }

    #[test]
    fn portable_robot_dna_keeps_existing_display_export_in_a_mixed_batch() {
        use serde_json::json;

        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world
            .spawn(LegacyDna::parse("start stop").unwrap())
            .unwrap();
        let expected = json!({"dna": world.export_dna(id).unwrap()});
        let (status, result) = classic_test_batch(
            &mut world,
            json!([
                {"type":"read_robot_dna","slot":id.slot(),"generation":id.generation(),
                 "marker_position":1},
                {"type":"export_dna","slot":id.slot(),"generation":id.generation()},
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        let result: serde_json::Value = serde_json::from_slice(&result).unwrap();
        assert_eq!(result["results"][0], expected);
        assert_eq!(result["results"][1], expected);
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

    #[test]
    fn native_robot_info_reproduction_command_returns_child_or_null() {
        for blocked in [false, true] {
            let mut world = Engine::new_native(EngineConfig {
                world_width: 8_000.0,
                world_height: 6_000.0,
                fixed_bot_radii: true,
                ..EngineConfig::testing()
            })
            .unwrap();
            let dna = LegacyDna::parse("start stop").unwrap();
            let parent = world.spawn_at(dna.clone(), [1_000.0; 2]).unwrap();
            world.set_organism_aim(parent, 0).unwrap();
            if blocked {
                world.spawn_at(dna, [1_120.0, 1_000.0]).unwrap();
            }
            let (status, response) = classic_test_batch(
                &mut world,
                serde_json::json!([{
                    "type": "reproduce_native_robot_info",
                    "slot": parent.slot(),
                    "generation": parent.generation(),
                }]),
            );
            assert_eq!(status, DbStatus::Ok);
            let reply: serde_json::Value = serde_json::from_slice(&response).unwrap();
            if blocked {
                assert!(reply["results"][0].is_null());
                assert_eq!(world.snapshot().organisms.len(), 2);
                assert!(world.snapshot().ties.is_empty());
            } else {
                let child = &world.snapshot().organisms[1];
                assert_eq!(
                    reply["results"][0],
                    serde_json::json!({
                        "slot": child.id.slot(),
                        "generation": child.id.generation(),
                    })
                );
                assert_eq!(child.position, [1_120.0, 1_000.0]);
                assert_eq!(world.snapshot().ties[0].last, 100);
            }
            assert_eq!(world.snapshot().tick, 0);
        }
    }

    #[test]
    fn native_robot_info_tag_updates_only_the_focused_robot_and_round_trips() {
        use crate::native_binary::{NativeRecordFile, NativeValue};

        let mut fixture = NativeRecordFile::decode(include_bytes!(
            "../tests/fixtures/db24832-movement-start-48032.sim"
        ))
        .unwrap();
        let retained_tag = format!("{:<45}12345", "Original description");
        let first = fixture.robots.first_mut().unwrap();
        first
            .set_value("tag", NativeValue::Bytes(retained_tag.as_bytes().to_vec()))
            .unwrap();
        let mut peer = first.clone();
        peer.set_value("old_slot", NativeValue::Word(2)).unwrap();
        peer.set_value("absolute_id", NativeValue::Long(2)).unwrap();
        fixture.robots.push(peer);
        let mut world = Engine::from_native_sim(&fixture.encode().unwrap()).unwrap();
        let focused = world.snapshot().organisms[0].id;
        let peer = world.snapshot().organisms[1].id;
        let read = |id: OrganismId| {
            serde_json::json!({
                "type": "read_robot_tag", "slot": id.slot(), "generation": id.generation(),
            })
        };
        let set = |tag: &str| {
            serde_json::json!({
                "type": "set_robot_tag", "slot": focused.slot(), "generation": focused.generation(), "tag": tag,
            })
        };
        let before = SaveFile::encode(&world).unwrap();
        let publications = world.snapshot_publication_count();
        let (status, reply) = classic_test_batch(&mut world, serde_json::json!([read(focused)]));
        assert_eq!(status, DbStatus::Ok);
        let reply: serde_json::Value = serde_json::from_slice(&reply).unwrap();
        assert_eq!(reply["results"][0], "Original description");
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
        assert_eq!(world.snapshot_publication_count(), publications);

        let entered = format!("Café ÿ\r{}", "Z".repeat(60));
        let normalized = format!("Caf? ÿ?{}", "Z".repeat(38));
        let stale = OrganismId::new(focused.slot(), focused.generation() + 1);
        let (status, _) =
            classic_test_batch(&mut world, serde_json::json!([set(&entered), read(stale)]));
        assert_ne!(status, DbStatus::Ok);
        assert_eq!(SaveFile::encode(&world).unwrap(), before);

        let (status, reply) = classic_test_batch(
            &mut world,
            serde_json::json!([set(&entered), read(focused), read(peer)]),
        );
        assert_eq!(status, DbStatus::Ok);
        let reply: serde_json::Value = serde_json::from_slice(&reply).unwrap();
        assert_eq!(
            reply["results"],
            serde_json::json!([normalized, normalized, "Original description"])
        );
        assert_eq!(world.snapshot().tick, 0);

        let mut resumed = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        assert_eq!(resumed.read_robot_tag(focused).unwrap(), normalized);
        let mut native_tag = b"Caf? \xff?".to_vec();
        native_tag.extend([b'Z'; 38]);
        native_tag.extend([b' '; 5]);
        let sim = resumed.export_native_sim().unwrap();
        let records = NativeRecordFile::decode(&sim).unwrap();
        assert_eq!(records.robots[0].bytes("tag").unwrap(), native_tag);
        assert_eq!(
            records.robots[1].bytes("tag").unwrap(),
            retained_tag.as_bytes()
        );
        let reopened = Engine::from_native_sim(&sim).unwrap();
        assert_eq!(
            reopened
                .read_robot_tag(reopened.snapshot().organisms[0].id)
                .unwrap(),
            normalized
        );

        let dbo = resumed.export_native_organisms(&[focused]).unwrap();
        let records = NativeRecordFile::decode(&dbo).unwrap();
        assert_eq!(records.robots[0].bytes("tag").unwrap(), native_tag);
        let mut imported = Engine::new_native(EngineConfig::testing()).unwrap();
        assert_eq!(imported.import_native_organisms(&dbo, None).unwrap(), 1);
        assert_eq!(
            imported
                .read_robot_tag(imported.snapshot().organisms[0].id)
                .unwrap(),
            normalized
        );

        // Both a blank accepted entry and original InputBox Cancel assign empty.
        let (status, reply) = classic_test_batch(
            &mut resumed,
            serde_json::json!([set(""), read(focused), read(peer)]),
        );
        assert_eq!(status, DbStatus::Ok);
        let reply: serde_json::Value = serde_json::from_slice(&reply).unwrap();
        assert_eq!(
            reply["results"],
            serde_json::json!(["", "", "Original description"])
        );
        let packed = resumed.export_native_organisms(&[focused]).unwrap();
        let records = NativeRecordFile::decode(&packed).unwrap();
        assert_eq!(records.robots[0].bytes("tag").unwrap(), &[b' '; 50]);
        assert!(!resumed.export_dna(focused).unwrap().contains("'#tag:"));
    }

    #[test]
    fn native_redraw_dispatches_each_tick_and_can_follow_a_pre_redraw_save() {
        let source = include_bytes!("../tests/fixtures/db24832-original-user.sim");
        let original = || Engine::from_native_sim(source).unwrap();
        let mut batched = original();
        let (status, _) = classic_test_batch(
            &mut batched,
            serde_json::json!([
                {"type":"tick", "count":2, "native_redraw":"every_cycle"}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);

        let mut separate = original();
        let (status, _) = classic_test_batch(
            &mut separate,
            serde_json::json!([
                {"type":"tick", "count":1}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        let before_redraw = separate.export_native_sim().unwrap();
        let (status, _) = classic_test_batch(
            &mut separate,
            serde_json::json!([
                {"type":"native_redraw"}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        assert_eq!(separate.snapshot().native_cycle, Some(282));
        assert_ne!(separate.export_native_sim().unwrap(), before_redraw);
        let checkpoint = SaveFile::encode(&separate).unwrap();
        let mut resumed = SaveFile::decode(&checkpoint).unwrap();
        let (status, _) = classic_test_batch(
            &mut resumed,
            serde_json::json!([
                {"type":"tick", "count":1, "native_redraw":"every_cycle", "capture_deaths":true}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        assert_eq!(resumed.snapshot().native_cycle, Some(283));
        let expected = batched.export_native_sim().unwrap();
        assert_eq!(resumed.export_native_sim().unwrap(), expected);

        let (status, _) = classic_test_batch(
            &mut batched,
            serde_json::json!([
                {"type":"tick", "count":1, "native_redraw":"invalid"}
            ]),
        );
        assert_eq!(status, DbStatus::InvalidCommand);
        assert_eq!(batched.export_native_sim().unwrap(), expected);
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
    fn native_sun_toolbar_toggles_the_effective_feed_indicator_without_advancing() {
        let mut config = EngineConfig::testing();
        config.vegetation.daytime = false;
        config.vegetation.day_night_enabled = true;
        config.vegetation.sun_up_enabled = true;
        config.vegetation.sun_up_threshold = 500_000;
        config.vegetation.sun_threshold_mode = 0;
        let mut world = Engine::new_native(config).unwrap();
        assert_eq!(world.snapshot().native_sun_on, Some(true));
        world
            .spawn(LegacyDna::parse("start stop").unwrap())
            .unwrap();
        world.tick().unwrap();
        assert!(!world.configuration().vegetation.daytime);
        assert_eq!(world.snapshot().native_sun_on, Some(true));

        let save = SaveFile::encode(&world).unwrap();
        let mut world = SaveFile::decode(&save).unwrap();
        assert_eq!(world.snapshot().native_sun_on, Some(true));
        let mut expected = serde_json::to_value(&world).unwrap();
        expected["vegetation_runtime"]
            .as_object_mut()
            .unwrap()
            .remove("native_sun_override");
        for enabled in [false, true] {
            let (status, response) = classic_test_batch(
                &mut world,
                serde_json::json!([{ "type": "toggle_native_sun" }]),
            );
            assert_eq!(status, DbStatus::Ok);
            let value: serde_json::Value = serde_json::from_slice(&response).unwrap();
            assert_eq!(value["results"][0], enabled);
            assert_eq!(world.snapshot().native_sun_on, Some(enabled));
            expected["config"]["vegetation"]["daytime"] = enabled.into();
            assert_eq!(serde_json::to_value(&world).unwrap(), expected);
        }
    }

    #[test]
    fn native_sun_toolbar_keeps_the_existing_button_state_during_a_sim_load() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        assert_eq!(world.toggle_native_sun(), Some(false));
        let mut source = Engine::new_native(EngineConfig::testing()).unwrap();
        source
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
            .unwrap();
        let save = source.export_native_sim().unwrap();
        assert_eq!(
            db_engine_load(&mut world, save.as_ptr(), save.len()),
            DbStatus::Ok
        );
        assert!(world.configuration().vegetation.daytime);
        assert_eq!(world.snapshot().native_sun_on, Some(false));
        world.tick().unwrap();
        assert!(world.configuration().vegetation.daytime);
        assert_eq!(world.snapshot().native_sun_on, Some(true));
    }

    #[test]
    fn opt_in_death_records_follow_atomic_tick_commands_and_preserve_legacy_results() {
        let mut engine = Engine::new(EngineConfig {
            metabolism_cost: 1,
            recording: crate::RecordingSettings {
                dead_snapshot_enabled: true,
                ..Default::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = engine
            .spawn(LegacyDna::parse("start\nstop").unwrap())
            .unwrap();
        engine.set_organism_energy(id, 1).unwrap();
        let absolute_id = engine.snapshot().organisms[0].unique_id;
        let before = SaveFile::encode(&engine).unwrap();
        let (status, result) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"tick","count":3,"capture_deaths":true},
                {"type":"set_organism_energy","slot":id.slot(),"generation":id.generation(),"energy":500}
            ]),
        );
        assert_eq!(status, DbStatus::EngineError);
        assert!(result.is_empty());
        assert_eq!(before, SaveFile::encode(&engine).unwrap());

        let (status, result) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"tick","count":3,"capture_deaths":true},
                {"type":"tick","count":1}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        let result: serde_json::Value = serde_json::from_slice(&result).unwrap();
        let records = result["results"][0]["death_records"].as_array().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["id"], absolute_id);
        assert_eq!(records[0]["tick"], 1);
        assert!(result["results"][1].is_null());
        assert_eq!(result["tick"], 4);
    }

    fn arm_repopulation(world: &Engine, amount: u32, second_source: bool) -> Engine {
        // Set the first eligible attempt explicitly so this boundary test
        // does not depend on unrelated long-running native replay changes.
        let mut frame = SaveFile::encode(world).unwrap();
        let mut payload: serde_json::Value = serde_json::from_slice(&frame[10..]).unwrap();
        if second_source {
            payload["species"][1]["vegetable"] = true.into();
            payload["species"][1]["reseed"] = true.into();
        }
        payload["config"]["vegetation"]["start_chloroplasts"] = 0.into();
        payload["config"]["vegetation"]["minimum_chloroplast_equivalents"] = 1000.into();
        payload["config"]["vegetation"]["repopulation_amount"] = amount.into();
        payload["config"]["vegetation"]["repopulation_cooldown"] = 1.into();
        payload["vegetation_runtime"]["native_repopulation_counter"] = 0.into();
        let payload = serde_json::to_vec(&payload).unwrap();
        frame.truncate(6);
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(&payload);
        SaveFile::decode(&frame).unwrap()
    }

    #[test]
    fn native_graph_layout_commands_merge_partial_rows_and_reject_invalid_queries_atomically() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let initial = SaveFile::encode(&world).unwrap();
        let read = serde_json::json!({"type": "read_native_graph_layout"});
        let (status, response) = classic_test_batch(&mut world, serde_json::json!([read]));
        assert_eq!(status, DbStatus::Ok);
        assert_eq!(SaveFile::encode(&world).unwrap(), initial);
        let state: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(state["results"][0]["graphs"].as_array().unwrap().len(), 18);
        assert_eq!(
            state["results"][0]["graphs"][0],
            serde_json::json!({"graph_index": 0, "visible": false})
        );
        assert_eq!(
            state["results"][0]["custom_queries"],
            serde_json::json!(["", "", ""])
        );

        let position = serde_json::json!({
            "type": "update_native_graph_layout",
            "graphs": [{"graph_index": 0, "left": 0, "top": -320}],
            "custom_queries": [{"slot": 2, "expression": "1 2 +"}],
        });
        let visibility = serde_json::json!({
            "type": "update_native_graph_layout", "graphs": [{"graph_index": 0, "visible": true}],
        });
        let (status, _) = classic_test_batch(
            &mut world,
            serde_json::json!([position, {
                "type": "update_native_graph_layout", "custom_queries": [{"slot": 4, "expression": "invalid"}],
            }]),
        );
        assert_eq!(status, DbStatus::EngineError);
        assert_eq!(SaveFile::encode(&world).unwrap(), initial);

        let (status, response) =
            classic_test_batch(&mut world, serde_json::json!([position, visibility, read]));
        assert_eq!(status, DbStatus::Ok);
        let state: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(
            state["results"][2]["graphs"][0],
            serde_json::json!({
                "graph_index": 0, "visible": true, "left": 0, "top": -320,
            })
        );
        assert_eq!(
            state["results"][2]["custom_queries"],
            serde_json::json!(["", "1 2 +", ""])
        );
    }

    #[test]
    fn native_graph_recording_commands_persist_counters_and_reject_bad_indices_atomically() {
        use crate::native_binary::{NativeRecordFile, NativeSimulationTail};

        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let read = serde_json::json!([{"type": "read_native_graph_recording"}]);
        let original = SaveFile::encode(&world).unwrap();
        let (status, _) = classic_test_batch(&mut world, read.clone());
        assert_eq!(status, DbStatus::Ok);
        assert_eq!(SaveFile::encode(&world).unwrap(), original);

        let row = serde_json::json!({"graph_index": 0, "enabled": true, "counter": 2});
        let update = serde_json::json!({
            "type": "update_native_graph_recording",
            "run_stamp": "2026-11-21 06-01-02",
            "graphs": [row],
        });
        let (status, _) = classic_test_batch(
            &mut world,
            serde_json::json!([update, {
                "type": "update_native_graph_recording",
                "graphs": [{"graph_index": 18, "enabled": true, "counter": 1}],
            }]),
        );
        assert_eq!(status, DbStatus::EngineError);
        assert_eq!(SaveFile::encode(&world).unwrap(), original);

        let (status, _) = classic_test_batch(&mut world, serde_json::json!([update]));
        assert_eq!(status, DbStatus::Ok);
        let checkpoint = SaveFile::encode(&world).unwrap();
        let native = NativeRecordFile::decode(&world.export_native_sim().unwrap()).unwrap();
        let tail = NativeSimulationTail::decode(&native.tail).unwrap();
        assert_eq!(tail.bytes("sim_start").unwrap(), b"2026-11-21 06-01-02");
        assert_eq!(tail.integer("graph[0].counter").unwrap(), 2);
        assert_eq!(tail.integer("graph[0].save").unwrap(), -1);

        for mut restored in [
            SaveFile::decode(&checkpoint).unwrap(),
            SaveFile::decode(&native.encode().unwrap()).unwrap(),
        ] {
            let (status, response) = classic_test_batch(&mut restored, read.clone());
            assert_eq!(status, DbStatus::Ok);
            let result: serde_json::Value = serde_json::from_slice(&response).unwrap();
            assert_eq!(
                result["results"][0],
                serde_json::json!({
                    "run_stamp": "2026-11-21 06-01-02",
                    "graphs": [row],
                })
            );
        }
    }

    #[test]
    fn rebuilt_command_candidate_preserves_the_native_graph_restart_boundary() {
        let mut config = EngineConfig::testing();
        config.recording.sampling_interval = 1;
        let mut committed = Engine::new_native(config).unwrap();
        committed.tick().unwrap();
        committed.native_restarted_this_cycle = true;
        let settings = crate::GraphSamplingSettings::default();
        let archive = SaveFile::encode(&committed).unwrap();

        let mut loaded = SaveFile::decode(&archive).unwrap();
        assert!(loaded.feed_native_graph_sample(settings, true).is_some());

        let mut candidate = rebuild_archive_command_candidate(&committed).unwrap();
        assert!(candidate.feed_native_graph_sample(settings, true).is_none());
        assert_eq!(SaveFile::encode(&candidate).unwrap(), archive);
    }

    #[test]
    fn native_graph_recording_stream_keeps_all_due_post_cycle_samples_without_changing_history() {
        let mut config = EngineConfig::testing();
        config.recording.sampling_interval = 5;
        let mut world = Engine::new_native(config).unwrap();
        world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [500.0, 500.0])
            .unwrap();
        let original = SaveFile::encode(&world).unwrap();
        let read = serde_json::json!([{"type": "read_native_graph_sample"}]);
        let (status, output) = classic_test_batch(&mut world, read.clone());
        assert_eq!(status, DbStatus::Ok);
        let output: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(output["results"][0]["native_cycle"], -1);
        assert_eq!(SaveFile::encode(&world).unwrap(), original);

        let (status, reset) = classic_test_batch(
            &mut world,
            serde_json::json!([{"type": "read_native_graph_reset_sample"}]),
        );
        assert_eq!(status, DbStatus::Ok);
        let reset: serde_json::Value = serde_json::from_slice(&reset).unwrap();
        assert_eq!(reset["results"][0]["native_cycle"], -1);
        assert_eq!(
            reset["results"][0]["species_graphs"][0]["values"]["population"],
            1.0,
        );
        assert_eq!(SaveFile::encode(&world).unwrap(), original);

        let (status, before_first_due) = classic_test_batch(
            &mut world,
            serde_json::json!([
                {"type": "feed_native_graph_sample", "slow": true, "due_only": true},
                {"type": "feed_native_graph_sample"},
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        let before_first_due: serde_json::Value =
            serde_json::from_slice(&before_first_due).unwrap();
        assert!(before_first_due["results"][0].is_null());
        assert_eq!(before_first_due["results"][1], output["results"][0]);
        assert_eq!(SaveFile::encode(&world).unwrap(), original);

        let (status, output) = classic_test_batch(
            &mut world,
            serde_json::json!([{
                "type": "tick", "count": 11, "capture_graph_samples": true,
            }]),
        );
        assert_eq!(status, DbStatus::Ok);
        let output: serde_json::Value = serde_json::from_slice(&output).unwrap();
        let result = &output["results"][0];
        let graph_samples = result["graph_samples"].as_array().unwrap();
        assert_eq!(
            graph_samples
                .iter()
                .map(|sample| (
                    sample["tick"].as_u64().unwrap(),
                    sample["native_cycle"].as_i64().unwrap()
                ))
                .collect::<Vec<_>>(),
            [(1, 0), (6, 5), (11, 10)],
        );

        let (status, current) = classic_test_batch(&mut world, read);
        assert_eq!(status, DbStatus::Ok);
        let current: serde_json::Value = serde_json::from_slice(&current).unwrap();
        assert_eq!(graph_samples.last().unwrap(), &current["results"][0]);
        // Cycle 5 ages the robot from five to six, crossing the 0.1 graph rounding threshold.
        assert_eq!(
            graph_samples[1]["species_graphs"][0]["values"]["average_age"].as_f64(),
            Some(f64::from(0.1_f32)),
        );

        let mut reference = SaveFile::decode(&original).unwrap();
        let (status, legacy) = classic_test_batch(
            &mut reference,
            serde_json::json!([{"type": "tick", "count": 11}]),
        );
        assert_eq!(status, DbStatus::Ok);
        let legacy: serde_json::Value = serde_json::from_slice(&legacy).unwrap();
        assert!(legacy["results"][0].is_null());
        assert_eq!(
            SaveFile::encode(&world).unwrap(),
            SaveFile::encode(&reference).unwrap()
        );
    }

    #[test]
    fn strict_repopulation_waits_for_the_selected_source_and_retries_atomically() {
        const SIM: &[u8] = include_bytes!("../tests/fixtures/db24832-original-user.sim");
        const ALGA: &str = "start 19 .up store stop";
        let has_unresolved = |world: &mut Engine| {
            let before = SaveFile::encode(world).unwrap();
            let (status, response) = classic_test_batch(
                world,
                serde_json::json!([
                    {"type": "has_unresolved_native_repopulation_sources"}
                ]),
            );
            assert_eq!(status, DbStatus::Ok);
            assert_eq!(SaveFile::encode(world).unwrap(), before);
            let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
            response["results"][0].as_bool().unwrap()
        };
        let mut world = Engine::from_native_sim(SIM).unwrap();
        let strict = serde_json::json!([{
            "type": "tick", "count": 1, "require_native_repopulation_source": true,
            "capture_deaths": true, "recording_weight": 50, "capture_graph_samples": true
        }]);
        // Loading a stale path and running an ordinary cycle both succeed.
        assert!(has_unresolved(&mut world));
        let mut restored = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        assert!(has_unresolved(&mut restored));
        let (status, _) = classic_test_batch(&mut world, strict.clone());
        assert_eq!(status, DbStatus::Ok);
        assert_eq!(world.snapshot().native_cycle, Some(282));
        let (status, response) = classic_test_batch(
            &mut world,
            serde_json::json!([
                {"type": "read_native_restart_sources"}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        let manifest: serde_json::Value = serde_json::from_slice(&response).unwrap();
        let source = &manifest["results"][0]["sources"][0];
        assert_eq!(source["lineage_id"], 1);
        assert_eq!(source["filename"], "Alga_Minimalis_Chloroplastus.txt");
        assert!(!source["directory"].as_str().unwrap().is_empty());

        let mut world = arm_repopulation(&world, 1, false);
        assert_eq!(
            world
                .bind_native_restart_sources(
                    vec![crate::world::NativeRestartBinding {
                        lineage_id: 2,
                        dna: "start 17 .up store stop".into(),
                        selected_directory: None,
                    }],
                    false
                )
                .unwrap(),
            1
        );
        let folder = tempfile::tempdir().unwrap();
        let destination = folder.path().join("new").join("outbound");
        let resident = world
            .snapshot()
            .organisms
            .iter()
            .find(|robot| !robot.vegetable)
            .unwrap();
        world
            .add_file_teleporter(
                crate::FileTeleporter::new(10_000, resident.position, 1.0)
                    .with_paths("", &destination),
            )
            .unwrap();

        let before = SaveFile::encode(&world).unwrap();
        let reseeds = world.snapshot().stats.reseeds;
        let (status, _) = classic_test_batch(
            &mut world,
            serde_json::json!([{
                "type": "tick", "count": 2, "require_native_repopulation_source": true
            }]),
        );
        assert_eq!(status, DbStatus::InvalidCommand);
        assert_eq!(SaveFile::encode(&world).unwrap(), before);

        let (status, body) = classic_test_batch(&mut world, strict.clone());
        assert_eq!(status, DbStatus::EngineError);
        assert!(body.is_empty());
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
        assert_eq!(std::fs::read_dir(folder.path()).unwrap().count(), 0);

        let mut error = DbBuffer::default();
        assert_eq!(db_last_error_json(&mut error), DbStatus::Ok);
        let details: serde_json::Value =
            serde_json::from_slice(unsafe { slice::from_raw_parts(error.data, error.len) })
                .unwrap();
        assert_eq!(db_buffer_free(&mut error), DbStatus::Ok);
        assert_eq!(details["code"], "missing_native_repopulation_source");
        for key in ["lineage_id", "filename", "directory"] {
            assert_eq!(details[key], source[key]);
        }

        let (status, response) = classic_test_batch(
            &mut world,
            serde_json::json!([{
                "type": "bind_native_restart_sources", "preserve_existing": true,
                "sources": [{"lineage_id": 1, "dna": ALGA}]
            }]),
        );
        assert_eq!(status, DbStatus::Ok);
        let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(response["results"][0], 1);
        let state: serde_json::Value = serde_json::to_value(&world).unwrap();
        assert_eq!(
            state["ordinary_restart"]["imported_sources"]
                .as_object()
                .unwrap()
                .len(),
            2
        );
        assert!(!has_unresolved(&mut world));
        let mut restored = SaveFile::decode(&SaveFile::encode(&world).unwrap()).unwrap();
        assert!(!has_unresolved(&mut restored));
        let (status, response) = classic_test_batch(&mut world, strict);
        assert_eq!(status, DbStatus::Ok);
        let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert!(response["results"][0]["death_records"].is_array());
        assert!(response["results"][0]["graph_samples"].is_array());
        assert_eq!(world.snapshot().native_cycle, Some(283));
        assert!(world.snapshot().stats.reseeds > reseeds);
        let expected = world.load_dna_source(ALGA).unwrap();
        assert!(
            world
                .snapshot()
                .organisms
                .iter()
                .any(|robot| robot.vegetable
                    && world.dna(robot.id).unwrap().blocks() == expected.blocks())
        );
        assert_eq!(std::fs::read_dir(&destination).unwrap().count(), 1);
    }

    #[test]
    fn cancelled_lineages_replay_atomically_when_a_second_source_is_missing() {
        let bytes = include_bytes!("../tests/fixtures/db24832-original-user.sim");
        let mut world = Engine::from_native_sim(bytes).unwrap();
        world.tick().unwrap();
        assert_eq!(world.snapshot().native_cycle, Some(282));
        let mut world = arm_repopulation(&world, 3, true);
        let before = SaveFile::encode(&world).unwrap();
        let reseeds = world.snapshot().stats.reseeds;

        let attempt = |world: &mut Engine, cancelled: &[u64]| {
            classic_test_batch(
                world,
                serde_json::json!([{
                    "type": "tick", "count": 1, "require_native_repopulation_source": true,
                    "cancelled_native_repopulation_lineages": cancelled
                }]),
            )
            .0
        };
        let missing_lineage = || {
            let mut buffer = DbBuffer::default();
            assert_eq!(db_last_error_json(&mut buffer), DbStatus::Ok);
            let error: serde_json::Value =
                serde_json::from_slice(unsafe { slice::from_raw_parts(buffer.data, buffer.len) })
                    .unwrap();
            assert_eq!(db_buffer_free(&mut buffer), DbStatus::Ok);
            assert_eq!(error["code"], "missing_native_repopulation_source");
            error["lineage_id"].as_u64().unwrap()
        };
        assert_eq!(attempt(&mut world, &[]), DbStatus::EngineError);
        let first = missing_lineage();
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
        assert_eq!(attempt(&mut world, &[first]), DbStatus::EngineError);
        let second = missing_lineage();
        assert_ne!(first, second);
        assert_eq!(SaveFile::encode(&world).unwrap(), before);

        assert_eq!(attempt(&mut world, &[first, second]), DbStatus::Ok);
        assert_eq!(world.snapshot().native_cycle, Some(283));
        assert_eq!(world.snapshot().stats.reseeds, reseeds);
        assert!(!world.has_unresolved_native_repopulation_sources().unwrap());
    }

    #[test]
    fn live_field_size_is_atomic_with_other_settings() {
        let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
        let mut command = serde_json::json!({
            "type": "update_environment", "metabolism_cost": 5,
            "vegetable_energy_per_tick": 0, "sunlight_energy": 123,
            "gravity": [0, 0], "drag": 0, "brownian_motion": 0
        });
        let (status, _) = classic_test_batch(&mut engine, serde_json::json!([command]));
        assert_eq!(status, DbStatus::Ok);
        assert_eq!(engine.snapshot().world_size, [1000.0; 2]);
        assert_eq!(engine.configuration().native_field_size, None);

        let old = SaveFile::encode(&engine).unwrap();
        command["field_size"] = 26.into();
        command["metabolism_cost"] = 12.into();
        let (status, _) = classic_test_batch(&mut engine, serde_json::json!([command]));
        assert_eq!(status, DbStatus::EngineError);
        assert_eq!(SaveFile::encode(&engine).unwrap(), old);

        command["field_size"] = 5.into();
        let (status, _) = classic_test_batch(&mut engine, serde_json::json!([command]));
        assert_eq!(status, DbStatus::Ok);
        assert_eq!(engine.configuration().metabolism_cost, 12);
        assert_eq!(engine.configuration().native_field_size, Some(5));
        assert_eq!(engine.snapshot().world_size, [40_000.0, 30_000.0]);
    }

    #[test]
    fn cost_preset_only_command_preserves_costs_and_rejects_invalid_options_atomically() {
        let mut world = Engine::new_native(EngineConfig::testing()).unwrap();
        let costs = world.cost_settings().clone();
        let (status, _) = classic_test_batch(
            &mut world,
            serde_json::json!([
                {"type":"update_cost_settings", "preset":2}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        assert_eq!(world.configuration().native_cost_preset, Some(2));
        assert_eq!(world.cost_settings(), &costs);
        let before = SaveFile::encode(&world).unwrap();

        let (status, _) = classic_test_batch(
            &mut world,
            serde_json::json!([
                {"type":"update_cost_settings", "preset":0},
                {"type":"update_cost_settings", "preset":3, "costs":{"movement":123}}
            ]),
        );
        assert_eq!(status, DbStatus::EngineError);
        assert_eq!(SaveFile::encode(&world).unwrap(), before);

        let (status, _) = classic_test_batch(
            &mut world,
            serde_json::json!([
                {"type":"update_cost_settings", "costs":{"movement":5}}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        assert_eq!(world.configuration().native_cost_preset, Some(2));
        assert_eq!(world.cost_settings().movement, 5.0);
    }

    #[test]
    fn native_console_resolves_the_focused_robots_literal_private_memory_names() {
        let mut engine = Engine::new_native(EngineConfig::testing()).unwrap();
        let first = engine
            .load_dna_source(concat!(
                "def Counter 901\n",
                "def Counter 902\n",
                "def up 903\n",
                "def Counter% 904\n",
                "def nrg 0\n",
                "start stop"
            ))
            .unwrap();
        let first = engine.spawn_at(first, [400.0, 500.0]).unwrap();
        let second = engine
            .load_dna_source("def Counter 911\nstart stop")
            .unwrap();
        let second = engine.spawn_at(second, [600.0, 500.0]).unwrap();
        let before = SaveFile::encode(&engine).unwrap();
        let (status, response) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"resolve_memory_address", "name":".Counter", "focus":first},
                {"type":"resolve_memory_address", "name":".Counter", "focus":second},
                {"type":"resolve_memory_address", "name":".up", "focus":first},
                {"type":"resolve_memory_address", "name":".UP", "focus":first},
                {"type":"resolve_memory_address", "name":".Counter%", "focus":first},
                {"type":"resolve_memory_address", "name":"908%", "focus":first},
                {"type":"resolve_memory_address", "name":"909suffix", "focus":first},
                {"type":"resolve_memory_address", "name":".up"},
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
        let addresses: Vec<_> = response["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|result| result["address"].as_i64().unwrap())
            .collect();
        assert_eq!(addresses, [902, 911, 903, 1, 904, 908, 909, 1]);

        for name in [".COUNTER", ".Counter!", "Counter", "..up", ".nrg", "908$"] {
            let (status, _) = classic_test_batch(
                &mut engine,
                serde_json::json!([{"type":"resolve_memory_address", "name": name, "focus":first}]),
            );
            assert_eq!(status, DbStatus::EngineError, "{name}");
        }
        let stale = OrganismId::new(first.slot(), first.generation() + 1);
        let (status, _) = classic_test_batch(
            &mut engine,
            serde_json::json!([{"type":"resolve_memory_address", "name":".Counter", "focus":stale}]),
        );
        assert_eq!(status, DbStatus::EngineError);
        assert_eq!(before, SaveFile::encode(&engine).unwrap());
    }

    fn populated_native_cpu_candidate_fixture() -> Engine {
        let original = include_bytes!("../tests/fixtures/db24832-original-user.sim");
        let mut engine = Engine::from_native_sim(original).unwrap();
        assert_eq!(engine.backend(), crate::BackendKind::Cpu);
        assert!(engine.population() > 100);
        engine.set_native_gaussian_spare(Some(1.25)).unwrap();
        engine.set_native_sun_change(22);
        engine
            .set_native_process_caches(native_cache_fixture(0.375, 421, 29, 199, 17))
            .unwrap();
        engine
            .set_native_restart_inputs(125, Some([1.125, 1.375]))
            .unwrap();
        engine.tick().unwrap();
        engine
    }

    #[test]
    fn cpu_command_candidate_clones_native_state_and_current_inspection_without_a_reload() {
        let engine = populated_native_cpu_candidate_fixture();
        let before = SaveFile::encode(&engine).unwrap();
        let mut candidate = engine.fork_cpu_command_candidate().unwrap().unwrap();
        assert_eq!(SaveFile::encode(&candidate).unwrap(), before);
        assert_eq!(
            serde_json::to_value(&candidate).unwrap(),
            serde_json::to_value(&engine).unwrap()
        );
        assert_eq!(
            serde_json::to_value(candidate.snapshot()).unwrap(),
            serde_json::to_value(engine.snapshot()).unwrap()
        );
        let selected = candidate.snapshot().organisms[0].id;
        candidate.set_organism_energy(selected, 777).unwrap();
        assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        assert_ne!(SaveFile::encode(&candidate).unwrap(), before);
    }

    #[test]
    fn cpu_command_candidate_clocked_ticks_match_the_serialized_reference_state() {
        let mut engine = populated_native_cpu_candidate_fixture();
        // This is the previous generic batch transaction's save/restore path.
        let mut reference = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
        reference
            .set_native_restart_inputs(162, Some([2.125, 2.375]))
            .unwrap();
        reference.tick_many(2).unwrap();
        let (status, response) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"set_native_restart_seed","seed":162,"skin_timer_seconds":[2.125,2.375]},
                {"type":"tick","count":2}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
        assert_eq!(response["tick"], engine.snapshot().tick);
        assert_eq!(response["results"], serde_json::json!([null, null]));
        assert_eq!(
            SaveFile::encode(&engine).unwrap(),
            SaveFile::encode(&reference).unwrap()
        );
    }

    #[test]
    fn cpu_command_candidate_preserves_session_inspection_but_real_load_resets_transients() {
        let mut continuous = Engine::new_native(EngineConfig::testing()).unwrap();
        continuous
            .spawn_at(
                LegacyDna::parse("start 1 .up store stop").unwrap(),
                [500.0, 500.0],
            )
            .unwrap();
        let (status, _) = classic_test_batch(
            &mut continuous,
            serde_json::json!([
                {"type":"set_native_restart_seed","seed":125,"skin_timer_seconds":[1.125,1.375]},
                {"type":"tick","count":10}
            ]),
        );
        assert_eq!(status, DbStatus::Ok);
        assert_eq!(
            continuous.snapshot().organisms[0].activation_history.len(),
            10
        );
        assert!(
            !continuous.snapshot().organisms[0]
                .activation_bitmap
                .is_empty()
        );

        let checkpoint = SaveFile::encode(&continuous).unwrap();
        let mut resumed = SaveFile::decode(&checkpoint).unwrap();
        // DB3S intentionally omits these three inspector-only program fields.
        // Saving does not clear the live session; actually loading starts a new
        // inspection history without changing persisted simulation state.
        assert_eq!(
            continuous.snapshot().organisms[0].activation_history.len(),
            10
        );
        assert!(
            resumed.snapshot().organisms[0]
                .activation_history
                .is_empty()
        );
        assert!(resumed.snapshot().organisms[0].activation_bitmap.is_empty());
        assert!(resumed.snapshot().organisms[0].debug_text.is_empty());
        assert_eq!(SaveFile::encode(&resumed).unwrap(), checkpoint);

        for engine in [&mut continuous, &mut resumed] {
            let (status, _) = classic_test_batch(
                engine,
                serde_json::json!([
                    {"type":"set_native_restart_seed","seed":125,"skin_timer_seconds":[1.125,1.375]},
                    {"type":"tick","count":3}
                ]),
            );
            assert_eq!(status, DbStatus::Ok);
        }
        let live = &continuous.snapshot().organisms[0];
        let loaded = &resumed.snapshot().organisms[0];
        assert_eq!(live.activation_history.len(), 13);
        assert_eq!(loaded.activation_history.len(), 3);
        assert_eq!(
            &live.activation_history[10..],
            loaded.activation_history.as_slice()
        );
        assert_eq!(live.activation_bitmap, loaded.activation_bitmap);
        assert_eq!(live.debug_text, loaded.debug_text);
        assert_eq!(
            SaveFile::encode(&continuous).unwrap(),
            SaveFile::encode(&resumed).unwrap()
        );

        let without_session_history = |engine: &Engine| {
            let mut organisms = serde_json::to_value(&engine.snapshot().organisms).unwrap();
            for organism in organisms.as_array_mut().unwrap() {
                assert!(
                    organism
                        .as_object_mut()
                        .unwrap()
                        .remove("activation_history")
                        .is_some()
                );
            }
            organisms
        };
        assert_eq!(
            without_session_history(&continuous),
            without_session_history(&resumed)
        );
    }

    #[test]
    fn cpu_command_candidate_keeps_a_late_failure_atomic_after_native_clock_edit_and_tick() {
        let mut engine = populated_native_cpu_candidate_fixture();
        let before = SaveFile::encode(&engine).unwrap();
        let snapshot_before = serde_json::to_value(engine.snapshot()).unwrap();
        let selected = engine.snapshot().organisms[0].id;
        let (status, response) = classic_test_batch(
            &mut engine,
            serde_json::json!([
                {"type":"set_native_restart_seed","seed":999,"skin_timer_seconds":[5,6]},
                {"type":"set_organism_energy","slot":selected.slot(),"generation":selected.generation(),"energy":42},
                {"type":"tick","count":1},
                {"type":"export_species_template_dna","species":4294967295u32}
            ]),
        );
        assert_ne!(status, DbStatus::Ok);
        assert!(response.is_empty());
        assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        assert_eq!(
            serde_json::to_value(engine.snapshot()).unwrap(),
            snapshot_before
        );
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
