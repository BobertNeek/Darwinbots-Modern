use crate::{Engine, EngineConfig, LegacyDna, OrganismId, SpeciesDefinition};
use crate::persistence::SaveFile;
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
#[serde(tag = "type", rename_all = "snake_case")]
enum EngineCommand {
    Tick { count: u32 },
    ImportDna { dna: String, position: Option<[f32; 2]> },
    Remove { slot: u32, generation: u32 },
    ImportSpecies {
        dna: String,
        name: String,
        vegetable: bool,
        color: u32,
        minimum_population: usize,
        reseed: bool,
        #[serde(default)]
        mutation_rate: f32,
        initial_energy: i32,
        positions: Vec<[f32; 2]>,
    },
    MoveOrganism { slot: u32, generation: u32, position: [f32; 2] },
    CloneOrganism { slot: u32, generation: u32, position: [f32; 2] },
    ReplaceDna { slot: u32, generation: u32, dna: String },
    ExportDna { slot: u32, generation: u32 },
    ManualReproduce {
        first_slot: u32,
        first_generation: u32,
        second_slot: Option<u32>,
        second_generation: Option<u32>,
        position: [f32; 2],
    },
    SwitchBackend { backend: crate::BackendPreference },
    AddObstacle { id: u32, minimum: [f32; 2], maximum: [f32; 2] },
    RemoveObstacle { id: u32 },
    AddTeleporter { id: u32, center: [f32; 2], radius: f32, destination: [f32; 2] },
    RemoveTeleporter { id: u32 },
    SetBrownianMotion { value: f32 },
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
        auto_speciation: Option<bool>,
        #[serde(default)]
        speciation_genetic_distance_percent: Option<f32>,
        #[serde(default)]
        toroidal_world: Option<bool>,
    },
}

impl Default for DbBuffer {
    fn default() -> Self {
        Self { data: ptr::null_mut(), len: 0, capacity: 0 }
    }
}

thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn db_api_version() -> u32 {
    ABI_VERSION
}

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
    ffi_guard("create engine", || db_engine_create_impl(config_data, config_len, out_engine))
}

fn db_engine_create_impl(config_data: *const u8, config_len: usize, out_engine: *mut *mut Engine) -> DbStatus {
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
    let bytes = if dna_len == 0 { &[] } else { unsafe { slice::from_raw_parts(dna_data, dna_len) } };
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

#[unsafe(no_mangle)]
pub extern "C" fn db_engine_snapshot_json(engine: *const Engine, out_buffer: *mut DbBuffer) -> DbStatus {
    let (Some(engine), Some(out_buffer)) = (unsafe { engine.as_ref() }, unsafe { out_buffer.as_mut() }) else {
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
    let (Some(engine), Some(out_buffer)) = (unsafe { engine.as_mut() }, unsafe { out_buffer.as_mut() }) else {
        return DbStatus::NullPointer;
    };
    if command_data.is_null() || command_len == 0 { return fail(DbStatus::InvalidCommand, "command batch is empty"); }
    if !out_buffer.data.is_null() { return fail(DbStatus::EngineError, "output buffer must be empty"); }
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
        return fail(DbStatus::InvalidCommand, format!("unsupported command batch version {}", batch.version));
    }
    let mut command_results = Vec::with_capacity(batch.commands.len());
    for command in batch.commands {
        let result = match command {
            EngineCommand::Tick { count } => {
                if count > 1_000_000 { return fail(DbStatus::InvalidCommand, "tick count exceeds batch limit"); }
                engine.tick_many(count).map(|_| serde_json::Value::Null)
            }
            EngineCommand::ImportDna { dna, position } => LegacyDna::parse(&dna).and_then(|dna| {
                let compatibility_warnings = dna.compatibility_warnings().to_vec();
                engine.spawn_at(dna, position.unwrap_or([0.0; 2])).map(|id| serde_json::json!({
                    "slot": id.slot(),
                    "generation": id.generation(),
                    "compatibility_warnings": compatibility_warnings,
                }))
            }),
            EngineCommand::Remove { slot, generation } => engine.remove(OrganismId::new(slot, generation))
                .map(|_| serde_json::Value::Null),
            EngineCommand::ImportSpecies {
                dna,
                name,
                vegetable,
                color,
                minimum_population,
                reseed,
                mutation_rate,
                initial_energy,
                positions,
            } => LegacyDna::parse(&dna).and_then(|dna| {
                let compatibility_warnings = dna.compatibility_warnings().to_vec();
                let species = engine.register_species(SpeciesDefinition {
                    name,
                    vegetable,
                    color,
                    minimum_population,
                    reseed,
                    mutation_rate,
                    ..SpeciesDefinition::default()
                });
                engine.spawn_species_batch(dna, species, positions, initial_energy).map(|ids| serde_json::json!({
                    "species": species.0,
                    "count": ids.len(),
                    "compatibility_warnings": compatibility_warnings,
                }))
            }),
            EngineCommand::MoveOrganism { slot, generation, position } =>
                engine.move_organism(OrganismId::new(slot, generation), position)
                    .map(|_| serde_json::Value::Null),
            EngineCommand::CloneOrganism { slot, generation, position } =>
                engine.clone_organism(OrganismId::new(slot, generation), position)
                    .map(|id| serde_json::json!({ "slot": id.slot(), "generation": id.generation() })),
            EngineCommand::ReplaceDna { slot, generation, dna } => LegacyDna::parse(&dna)
                .and_then(|dna| engine.replace_dna(OrganismId::new(slot, generation), dna))
                .map(|_| serde_json::Value::Null),
            EngineCommand::ExportDna { slot, generation } => engine.export_dna(OrganismId::new(slot, generation))
                .map(|dna| serde_json::json!({ "dna": dna })),
            EngineCommand::ManualReproduce {
                first_slot,
                first_generation,
                second_slot,
                second_generation,
                position,
            } => {
                let second = second_slot.zip(second_generation)
                    .map(|(slot, generation)| OrganismId::new(slot, generation));
                engine.manual_reproduce(OrganismId::new(first_slot, first_generation), second, position)
                    .map(|id| serde_json::json!({ "slot": id.slot(), "generation": id.generation() }))
            }
            EngineCommand::SwitchBackend { backend } => engine.switch_backend(backend)
                .map(|_| serde_json::json!({ "backend": format!("{:?}", engine.backend()) })),
            EngineCommand::AddObstacle { id, minimum, maximum } =>
                engine.add_obstacle(crate::Obstacle { id, minimum, maximum }).map(|_| serde_json::Value::Null),
            EngineCommand::RemoveObstacle { id } => engine.remove_obstacle(id).map(|_| serde_json::Value::Null),
            EngineCommand::AddTeleporter { id, center, radius, destination } =>
                engine.add_teleporter(crate::Teleporter { id, center, radius, destination }).map(|_| serde_json::Value::Null),
            EngineCommand::RemoveTeleporter { id } => engine.remove_teleporter(id).map(|_| serde_json::Value::Null),
            EngineCommand::SetBrownianMotion { value } => engine.set_brownian_motion(value).map(|_| serde_json::Value::Null),
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
                auto_speciation,
                speciation_genetic_distance_percent,
                toroidal_world,
            } => engine
                .update_environment(
                    metabolism_cost,
                    vegetable_energy_per_tick,
                    sunlight_energy,
                    gravity,
                    drag,
                    brownian_motion,
                )
                .and_then(|_| engine.update_db2_settings(physics, shots, vegetation))
                .and_then(|_| engine.update_speciation_settings(
                    auto_speciation,
                    speciation_genetic_distance_percent,
                ))
                .map(|_| {
                    if let Some(enabled) = toroidal_world {
                        engine.set_toroidal_world(enabled);
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
    match serde_json::to_vec(&result) {
        Ok(bytes) => write_buffer(out_buffer, bytes),
        Err(error) => fail(DbStatus::EngineError, error),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn db_engine_capabilities_json(engine: *const Engine, out_buffer: *mut DbBuffer) -> DbStatus {
    let (Some(engine), Some(out_buffer)) = (unsafe { engine.as_ref() }, unsafe { out_buffer.as_mut() }) else {
        return DbStatus::NullPointer;
    };
    if !out_buffer.data.is_null() { return fail(DbStatus::EngineError, "output buffer must be empty"); }
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

#[unsafe(no_mangle)]
pub extern "C" fn db_engine_save(engine: *const Engine, out_buffer: *mut DbBuffer) -> DbStatus {
    let (Some(engine), Some(out_buffer)) = (unsafe { engine.as_ref() }, unsafe { out_buffer.as_mut() }) else {
        return DbStatus::NullPointer;
    };
    if !out_buffer.data.is_null() { return fail(DbStatus::EngineError, "output buffer must be empty"); }
    match SaveFile::encode(engine) {
        Ok(bytes) => write_buffer(out_buffer, bytes),
        Err(error) => fail(DbStatus::EngineError, error),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn db_engine_load(engine: *mut Engine, save_data: *const u8, save_len: usize) -> DbStatus {
    ffi_guard("load engine", || db_engine_load_impl(engine, save_data, save_len))
}

fn db_engine_load_impl(engine: *mut Engine, save_data: *const u8, save_len: usize) -> DbStatus {
    let Some(engine) = (unsafe { engine.as_mut() }) else { return DbStatus::NullPointer };
    if save_data.is_null() || save_len == 0 { return fail(DbStatus::EngineError, "save buffer is empty"); }
    let bytes = unsafe { slice::from_raw_parts(save_data, save_len) };
    match SaveFile::decode(bytes) {
        Ok(loaded) => {
            *engine = loaded;
            DbStatus::Ok
        }
        Err(error) => fail(DbStatus::EngineError, error),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn db_last_error_json(out_buffer: *mut DbBuffer) -> DbStatus {
    let Some(out_buffer) = (unsafe { out_buffer.as_mut() }) else { return DbStatus::NullPointer };
    if !out_buffer.data.is_null() { return DbStatus::EngineError; }
    let message = LAST_ERROR.with(|last_error| last_error.borrow().clone());
    match serde_json::to_vec(&serde_json::json!({ "version": ABI_VERSION, "message": message })) {
        Ok(bytes) => write_buffer(out_buffer, bytes),
        Err(_) => DbStatus::EngineError,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn db_buffer_free(buffer: *mut DbBuffer) -> DbStatus {
    let Some(buffer) = (unsafe { buffer.as_mut() }) else {
        return DbStatus::NullPointer;
    };
    if !buffer.data.is_null() {
        unsafe { drop(Vec::from_raw_parts(buffer.data, buffer.len, buffer.capacity)) };
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
            let detail = payload.downcast_ref::<&str>().copied()
                .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                .unwrap_or("unknown native panic");
            fail(DbStatus::EngineError, format!("{operation} panicked: {detail}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_guard_converts_panics_to_structured_engine_errors() {
        let status = ffi_guard("test operation", || panic!("deliberate failure"));
        assert_eq!(status, DbStatus::EngineError);
        let message = LAST_ERROR.with(|error| error.borrow().clone());
        assert!(message.contains("test operation panicked"));
        assert!(message.contains("deliberate failure"));
    }
}
