use darwinbots_engine::{DnaCosts, Engine, EngineConfig, LegacyDna, OrganismId, SaveFile};
use serde_json::{Value, json};

fn restore_fields(engine: &Engine, version: u16, edit: impl FnOnce(&mut Value)) -> Engine {
    let save = SaveFile::encode(engine).unwrap();
    let mut payload: Value = serde_json::from_slice(&save[10..]).unwrap();
    edit(&mut payload);
    let bytes = serde_json::to_vec(&payload).unwrap();
    let mut framed = b"DB3S".to_vec();
    framed.extend_from_slice(&version.to_le_bytes());
    framed.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    framed.extend_from_slice(&bytes);
    SaveFile::decode(&framed).unwrap()
}

fn idle_engine() -> (Engine, OrganismId) {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine
        .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0, 500.0])
        .unwrap();
    (engine, id)
}

#[test]
fn genuine_reference_single_values_survive_modern_save_restore_exactly() {
    // First robot in the user's genuine 2.48.32 test.sim, SHA256 09b77b6e...e8a55e.
    let (engine, id) = idle_engine();
    let engine = restore_fields(&engine, 4, |payload| {
        let materials = &mut payload["slots"][id.slot() as usize]["organism"]["materials"];
        materials["energy"] = json!(f32::from_bits(0x4412c0fd));
        materials["body"] = json!(f32::from_bits(0x4319f238));
        materials["waste"] = json!(f32::from_bits(0x41bf9b42));
    });
    let encoded = SaveFile::encode(&engine).unwrap();
    assert_eq!(u16::from_le_bytes([encoded[4], encoded[5]]), 4);
    let restored = SaveFile::decode(&encoded).unwrap();
    let materials = &restored.robot_state(id).unwrap().materials;
    assert_eq!(
        materials.energy.to_bits(),
        f32::from_bits(0x4412c0fd).to_bits()
    );
    assert_eq!(
        materials.body.to_bits(),
        f32::from_bits(0x4319f238).to_bits()
    );
    assert_eq!(
        materials.waste.to_bits(),
        f32::from_bits(0x41bf9b42).to_bits()
    );
    assert_eq!(restored.organism(id).unwrap().energy, 587);
    assert_eq!(restored.organism(id).unwrap().body, 154);
}

#[test]
fn fractional_upkeep_changes_the_store_before_an_integer_readback_changes() {
    let (mut engine, id) = idle_engine();
    engine
        .update_cost_settings(DnaCosts {
            dna_upkeep: 0.1,
            ..Default::default()
        })
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.robot_state(id).unwrap().materials.energy, 999.8_f32);
    assert_eq!(engine.memory_at(id, 310).unwrap(), 1_000);
    assert_eq!(engine.robot_state(id).unwrap().materials.waste, 0.0);
    let mut restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    engine.tick().unwrap();
    restored.tick().unwrap();
    assert_eq!(
        engine.robot_state(id).unwrap().materials.energy.to_bits(),
        restored.robot_state(id).unwrap().materials.energy.to_bits()
    );
}

#[test]
fn unmaking_fractional_shell_spends_the_native_conversion_cost() {
    let (engine, id) = idle_engine();
    let mut engine = restore_fields(&engine, 4, |payload| {
        payload["slots"][id.slot() as usize]["organism"]["materials"]["shell"] = json!(0.5);
    });
    engine
        .replace_dna(
            id,
            LegacyDna::parse("start -1 .mkshell store stop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    let materials = &engine.robot_state(id).unwrap().materials;
    assert_eq!(materials.shell, 0.0);
    assert_eq!(materials.energy, 999.95_f32);
    assert_eq!(materials.waste, 0.0);
}

#[test]
fn birth_preserves_fractional_energy_then_applies_the_native_half_unit_death_threshold() {
    let (mut engine, parent) = idle_engine();
    engine.set_organism_energy(parent, 1).unwrap();
    engine
        .replace_dna(
            parent,
            LegacyDna::parse("cond *.robage 1 < start 25 .repro store stop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|robot| robot.parents[0] == Some(parent))
        .unwrap()
        .id;
    assert_eq!(
        engine.robot_state(child).unwrap().materials.energy,
        0.24975_f32
    );
    assert_eq!(
        engine.robot_state(parent).unwrap().materials.energy,
        0.74975_f32
    );
    engine.tick().unwrap();
    assert_eq!(engine.population(), 1);
    assert!(engine.organism(child).is_err());
    assert_eq!(
        engine.robot_state(parent).unwrap().materials.energy,
        0.74975_f32
    );
}

#[test]
fn default_energy_ammunition_keeps_a_fraction_below_its_integer_value() {
    let (engine, id) = idle_engine();
    let mut engine = restore_fields(&engine, 4, |payload| {
        payload["slots"][id.slot() as usize]["organism"]["materials"]["energy"] = json!(0.75);
    });
    engine
        .replace_dna(id, LegacyDna::parse("start -2 .shoot store stop").unwrap())
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.robot_state(id).unwrap().materials.energy, 0.7425_f32);
    assert_eq!(engine.snapshot().shots[0].value, 0);
    assert_eq!(engine.snapshot().shots[0].energy, 0.0075_f32);
}

#[test]
fn v3_carried_fractions_are_absorbed_once_when_the_old_save_is_loaded() {
    let (engine, id) = idle_engine();
    let engine = restore_fields(&engine, 3, |payload| {
        let robot = &mut payload["slots"][id.slot() as usize]["organism"];
        robot["program"]["fractional_cost_remainder"] = json!(0.2);
        let materials = &mut robot["materials"];
        materials["energy"] = json!(999);
        materials["plant_energy_carry_micros"] = json!(300_000);
        materials["slime"] = json!(78);
        materials["slime_decay_fraction"] = json!(0.4);
        materials["chloroplasts"] = json!(200);
        materials["chloroplast_decay_carry_micros"] = json!(472_030);
    });
    assert_eq!(engine.robot_state(id).unwrap().materials.energy, 999.1_f32);
    assert_eq!(engine.robot_state(id).unwrap().materials.slime, 78.4_f32);
    assert_eq!(
        engine.robot_state(id).unwrap().materials.chloroplasts,
        199.52797_f32
    );
    let restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    assert_eq!(
        restored.robot_state(id).unwrap().materials,
        engine.robot_state(id).unwrap().materials
    );
}

#[test]
fn fractional_material_transaction_cost_is_energy_and_waste() {
    let (engine, id) = idle_engine();
    let mut engine = restore_fields(&engine, 4, |payload| {
        payload["slots"][id.slot() as usize]["organism"]["materials"]["shell"] = json!(0.5);
    });
    engine
        .update_cost_settings(DnaCosts {
            shell: 1.0,
            ..Default::default()
        })
        .unwrap();
    engine
        .replace_dna(
            id,
            LegacyDna::parse("start -1 .mkshell store stop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    let materials = &engine.robot_state(id).unwrap().materials;
    assert_eq!(materials.shell, 0.0);
    assert_eq!(materials.energy, 999.45_f32);
    assert_eq!(materials.waste, 0.5);
}

#[test]
fn dna_costs_round_the_native_store_after_each_instruction() {
    let (mut engine, id) = idle_engine();
    engine
        .replace_dna(id, LegacyDna::parse("start 1 2 3 stop").unwrap())
        .unwrap();
    engine
        .update_cost_settings(DnaCosts {
            number: 0.1,
            ..Default::default()
        })
        .unwrap();
    engine.tick().unwrap();
    let actual = engine.robot_state(id).unwrap().materials.energy;
    assert_eq!(actual, 999.700_1);
    assert_ne!(actual.to_bits(), (1000.0_f32 - 0.3).to_bits());
    assert_eq!(engine.memory_at(id, 310).unwrap(), 1000);
}
