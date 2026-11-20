use darwinbots_engine::{Engine, EngineConfig, LegacyDna, OrganismId, SaveFile};
use serde_json::{Value, json};

fn world(width: f32, height: f32) -> Engine {
    let mut engine = Engine::new(EngineConfig {
        world_width: width,
        world_height: height,
        metabolism_cost: 0,
        ..EngineConfig::testing()
    })
    .unwrap();
    engine.update_mutation_settings(false, 1.0, &[]).unwrap();
    engine
}

fn idle_bot(engine: &mut Engine, position: [f32; 2], aim: i32) -> OrganismId {
    let id = engine
        .spawn_at(LegacyDna::parse("start stop").unwrap(), position)
        .unwrap();
    engine.set_organism_aim(id, aim).unwrap();
    id
}

fn position_words(engine: &Engine, id: OrganismId) -> [i32; 2] {
    [219, 217].map(|address| engine.memory_at(id, address).unwrap())
}

fn reference_position_words(engine: &Engine, id: OrganismId) -> [i32; 2] {
    [689, 690].map(|address| engine.memory_at(id, address).unwrap())
}

fn restore_fields(engine: &Engine, edit: impl FnOnce(&mut Value)) -> Engine {
    let save = SaveFile::encode(engine).unwrap();
    let mut payload: Value = serde_json::from_slice(&save[10..]).unwrap();
    edit(&mut payload);
    let bytes = serde_json::to_vec(&payload).unwrap();
    let mut framed = save[..6].to_vec();
    framed.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    framed.extend_from_slice(&bytes);
    SaveFile::decode(&framed).unwrap()
}

fn set_saved_word(payload: &mut Value, id: OrganismId, address: u16, value: i32) {
    let cells = payload["slots"][id.slot() as usize]["organism"]["program"]["memory"]["cells"]
        .as_array_mut()
        .unwrap();
    cells.retain(|cell| cell[0].as_u64() != Some(u64::from(address)));
    if value != 0 {
        cells.push(json!([address, value]));
    }
}

#[test]
fn optical_position_reads_follow_slot_order_and_preserve_native_integer_words() {
    let mut engine = world(64_000.0, 96_000.0);
    let first = idle_bot(&mut engine, [40_001.0, 24_001.5], 0);
    let second = idle_bot(&mut engine, [40_351.0, 24_001.5], 628);
    let mut engine = restore_fields(&engine, |payload| {
        // Senses.lookoccurr copies target mem(219)/mem(217), before the current
        // robot's WriteSenses publishes its own coordinates. Native saved
        // Integer words can exceed DNA stores' +/-32000 range.
        for (id, words) in [(first, [111, 112]), (second, [32_767, -32_768])] {
            for (address, value) in [219, 217].into_iter().zip(words) {
                set_saved_word(payload, id, address, value);
            }
        }
    });
    assert_eq!(position_words(&engine, second), [32_767, -32_768]);
    let mut resumed = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();

    engine.tick().unwrap();
    resumed.tick().unwrap();

    assert!(engine.memory(first, "eye5").unwrap() > 0);
    assert!(engine.memory(second, "eye5").unwrap() > 0);
    assert_eq!(reference_position_words(&engine, first), [32_767, -32_768]);
    assert_eq!(reference_position_words(&engine, second), [20_000, 8_000]);
    assert_eq!(position_words(&engine, first), [20_000, 8_000]);
    assert_eq!(position_words(&engine, second), [20_176, 8_000]);
    assert_eq!(
        SaveFile::encode(&engine).unwrap(),
        SaveFile::encode(&resumed).unwrap()
    );

    // The cached reference words themselves must survive a save, even while
    // the target's own coordinates have already been refreshed.
    let mut restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    assert_eq!(
        reference_position_words(&restored, first),
        [32_767, -32_768]
    );
    engine.tick().unwrap();
    restored.tick().unwrap();
    assert_eq!(reference_position_words(&engine, first), [20_176, 8_000]);
    assert_eq!(
        SaveFile::encode(&engine).unwrap(),
        SaveFile::encode(&restored).unwrap()
    );
}

#[test]
fn large_world_spawn_move_clone_and_core_senses_use_independent_single_divisors() {
    let mut engine = world(64_000.0, 96_000.0);
    let id = idle_bot(&mut engine, [40_001.0, 24_001.5], 0);
    // main.frm stores xDivisor/yDivisor as Single: 2 and 3 here.
    // The scaled halves use VB's ties-to-even conversion, independently.
    assert_eq!(position_words(&engine, id), [20_000, 8_000]);

    engine.move_organism(id, [40_003.0, 24_004.5]).unwrap();
    assert_eq!(position_words(&engine, id), [20_002, 8_002]);
    engine.tick().unwrap();
    assert_eq!(position_words(&engine, id), [20_002, 8_002]);
    assert_eq!(
        engine.robot_state(id).unwrap().kinematics.position,
        [40_003.0, 24_004.5]
    );

    let cloned = engine.clone_organism(id, [32_001.0, 48_001.5]).unwrap();
    assert_eq!(position_words(&engine, cloned), [16_000, 16_000]);
    let mut restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    restored.tick().unwrap();
    assert_eq!(position_words(&restored, id), [20_002, 8_002]);
    assert_eq!(position_words(&restored, cloned), [16_000, 16_000]);
    assert_eq!(
        restored.robot_state(cloned).unwrap().kinematics.position,
        [32_001.0, 48_001.5]
    );
}

#[test]
fn native_position_mod_rounds_before_wrapping_and_uses_one_below_the_scale_threshold() {
    let mut engine = world(32_000.0, 32_000.0);
    let id = idle_bot(&mut engine, [500.5, 501.5], 0);
    // Exactly 32000 leaves each divisor at one. CInt rounds halves to even.
    assert_eq!(position_words(&engine, id), [500, 502]);
    engine.tick().unwrap();
    assert_eq!(position_words(&engine, id), [500, 502]);

    // VB Mod converts its operands to Long first: 31999.5 rounds to 32000,
    // then wraps to zero. Rounding a floating remainder after Mod is wrong.
    engine.move_organism(id, [31_999.5, 31_999.25]).unwrap();
    assert_eq!(position_words(&engine, id), [0, 31_999]);
    engine.move_organism(id, [32_000.0, 32_000.0]).unwrap();
    assert_eq!(position_words(&engine, id), [0, 0]);
    engine.move_organism(id, [-1.0, -1.0]).unwrap();
    assert_eq!(position_words(&engine, id), [0, 0]);
}

#[test]
fn large_world_position_mod_wraps_scaled_half_units_and_exact_world_edges() {
    let mut engine = world(64_000.0, 96_000.0);
    let id = idle_bot(&mut engine, [63_999.0, 95_998.5], 0);
    assert_eq!(position_words(&engine, id), [0, 0]);

    engine.move_organism(id, [63_998.5, 95_997.0]).unwrap();
    assert_eq!(position_words(&engine, id), [31_999, 31_999]);
    engine.move_organism(id, [64_000.0, 96_000.0]).unwrap();
    assert_eq!(position_words(&engine, id), [0, 0]);
    let restored = SaveFile::decode(&SaveFile::encode(&engine).unwrap()).unwrap();
    assert_eq!(position_words(&restored, id), [0, 0]);
    assert_eq!(
        restored.robot_state(id).unwrap().kinematics.position,
        [64_000.0, 96_000.0]
    );
}
