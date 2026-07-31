use darwinbots_engine::{Engine, EngineConfig, EngineError, LegacyDna, RobotState, SpeciesId};

fn inert_dna() -> LegacyDna {
    LegacyDna::parse("start\nstop").expect("inert DNA should parse")
}

#[test]
fn robot_state_contains_every_phase1_field() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine.spawn_at(inert_dna(), [10.0, 20.0]).unwrap();
    let RobotState {
        identity,
        flags,
        kinematics,
        materials,
        counters,
        program,
        senses,
        effects,
        heredity,
        ties,
        phenotype,
    } = engine.robot_state(id).unwrap();

    assert_eq!(identity.species, SpeciesId::default());
    assert_eq!(identity.parents, [None, None]);
    let _ = (
        &identity.species_file,
        &identity.owner,
        &identity.tag,
        identity.simulation_id,
        identity.absolute_id,
    );
    assert!(flags.exists);
    let _ = (
        flags.corpse,
        flags.dead,
        flags.vegetable,
        flags.fixed,
        flags.can_see,
        flags.can_execute_dna,
        flags.can_move,
        flags.can_reproduce,
        flags.can_mutate,
        flags.can_use_chloroplasts,
        flags.can_receive_viruses,
    );
    assert_eq!(kinematics.position, [10.0, 20.0]);
    let _ = (
        kinematics.previous_position,
        kinematics.velocity,
        kinematics.actual_velocity,
        kinematics.pending_velocity,
        kinematics.independent_impulse,
        kinematics.resistive_impulse,
        kinematics.aim_vector,
        kinematics.angular_momentum,
    );
    assert_eq!(materials.energy, 1_000);
    let _ = (
        materials.body,
        materials.mass,
        materials.radius,
        materials.chloroplasts,
        materials.shell,
        materials.slime,
        materials.venom,
        materials.poison,
        materials.waste,
        materials.permanent_waste,
    );
    assert_eq!(counters.age, 0);
    let _ = (
        counters.generation,
        counters.offspring,
        counters.descendants,
        counters.kills,
        counters.birth_cycle,
        counters.multibot,
    );
    assert_eq!(program.dna, inert_dna());
    assert_eq!(program.memory.read(310), 1_000);
    assert_eq!(program.private_variables.read(1), 0);
    let _ = (
        program.rng_state,
        &program.mutation_metadata,
        program.fractional_cost_remainder,
        &program.debug_text,
        program.current_gene,
        program.condition_count,
        &program.activation_bitmap,
    );
    let _ = (
        &senses.vision_reference_occurrences,
        senses.focused_opponent,
        senses.touch,
        senses.taste,
        senses.eye_configuration,
        senses.console_reference,
        senses.rgb_monitor,
    );
    let _ = (
        effects.poisoning_location,
        effects.poisoning_value,
        effects.poisoning_countdown,
        effects.paralysis_location,
        effects.paralysis_value,
        effects.paralysis_countdown,
        &effects.stored_virus,
        effects.virus_build_timer,
        &effects.stored_sperm,
        effects.fertilization_timer,
        effects.forced_reproduction,
    );
    assert_eq!(heredity.addresses, [0; 20]);
    assert_eq!(heredity.epigenetic_staging, [0; 15]);
    assert!(ties.iter().all(Option::is_none));
    let _ = phenotype;
}

#[test]
fn slot_reuse_changes_generation_without_reordering_live_slots() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let first = engine.spawn(inert_dna()).unwrap();
    let removed = engine.spawn(inert_dna()).unwrap();
    let third = engine.spawn(inert_dna()).unwrap();

    engine.remove(removed).unwrap();
    let reused = engine.spawn(inert_dna()).unwrap();

    assert_eq!(removed.slot(), reused.slot());
    assert_ne!(removed.generation(), reused.generation());
    assert_eq!(
        engine
            .snapshot()
            .organisms
            .iter()
            .map(|robot| robot.id)
            .collect::<Vec<_>>(),
        vec![first, reused, third]
    );
    assert!(matches!(
        engine.robot_state(removed),
        Err(EngineError::StaleOrganismId)
    ));
}

#[test]
fn physics_batch_round_trip_changes_only_kinematics() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let first = engine.spawn_at(inert_dna(), [10.0, 20.0]).unwrap();
    let second = engine.spawn_at(inert_dna(), [30.0, 40.0]).unwrap();
    let before = engine.robot_state(first).unwrap().clone();

    let mut batch = engine.physics_batch();
    assert_eq!(batch.ids, vec![first, second]);
    batch.batch.positions[0] = [15.0, 25.0];
    batch.batch.velocities[0] = [2.0, 3.0];
    engine.apply_physics_batch(batch).unwrap();

    let after = engine.robot_state(first).unwrap();
    assert_eq!(after.kinematics.position, [15.0, 25.0]);
    assert_eq!(after.kinematics.velocity, [2.0, 3.0]);
    let mut without_kinematics = after.clone();
    without_kinematics.kinematics = before.kinematics.clone();
    assert_eq!(without_kinematics, before);
}

#[test]
fn one_robot_has_one_writable_owner_for_energy_age_position_and_memory() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let id = engine.spawn_at(inert_dna(), [50.0, 60.0]).unwrap();

    engine.move_organism(id, [125.0, 250.0]).unwrap();
    engine.tick().unwrap();

    let state = engine.robot_state(id).unwrap();
    let snapshot = engine.organism(id).unwrap();
    assert_eq!(state.materials.energy, snapshot.energy);
    assert_eq!(state.counters.age, snapshot.age);
    assert_eq!(state.kinematics.position, snapshot.position);
    assert_eq!(state.program.memory.read(310), state.materials.energy);
    assert_eq!(engine.memory_at(id, 310).unwrap(), state.materials.energy);
    assert_eq!(
        state.program.memory.read(219),
        state.kinematics.position[0].round() as i32
    );
    assert_eq!(
        state.program.memory.read(217),
        state.kinematics.position[1].round() as i32
    );
}
