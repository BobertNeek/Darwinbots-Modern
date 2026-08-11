use std::collections::{BTreeMap, BTreeSet};

use darwinbots_engine::{
    CommandOutcome, CyclePhase, Engine, EngineConfig, LegacyDna, OrganismId, PhaseCommands,
    PhaseReadbacks, SysvarRole, SysvarRuntime, TieCommands, VirusCommands, VisionReadbacks,
    VmMemory, sysvar_address,
};

const CANONICAL_SYSVARS: &str = include_str!("../src/db2_sysvars_248.txt");

fn canonical_entries() -> Vec<(&'static str, u16)> {
    let mut lines = CANONICAL_SYSVARS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let mut entries = Vec::new();
    while let (Some(name), Some(address)) = (lines.next(), lines.next()) {
        entries.push((name, address.parse().unwrap()));
    }
    entries
}

#[test]
fn canonical_table_has_255_names_and_247_addresses() {
    let entries = canonical_entries();
    let addresses: BTreeSet<_> = entries.iter().map(|entry| entry.1).collect();

    assert_eq!(entries.len(), 255);
    assert_eq!(addresses.len(), 247);
}

#[test]
fn every_canonical_address_has_one_lifecycle_definition() {
    let runtime = SysvarRuntime::new();
    let canonical_addresses: BTreeSet<_> = canonical_entries()
        .into_iter()
        .map(|entry| entry.1)
        .collect();
    let defined_addresses: BTreeSet<_> = runtime
        .definitions()
        .iter()
        .map(|definition| definition.address)
        .collect();

    assert_eq!(runtime.definitions().len(), 247);
    assert_eq!(defined_addresses, canonical_addresses);
}

#[test]
fn aliases_share_identical_lifecycle() {
    let runtime = SysvarRuntime::new();
    let mut aliases = BTreeMap::<u16, Vec<&str>>::new();
    for (name, address) in canonical_entries() {
        aliases.entry(address).or_default().push(name);
    }

    for (address, names) in aliases.into_iter().filter(|(_, names)| names.len() > 1) {
        let expected = runtime.definition(address).unwrap();
        for name in names {
            let resolved = sysvar_address(name).unwrap() as u16;
            assert_eq!(resolved, address);
            assert_eq!(runtime.definition(resolved), Some(expected));
        }
    }
}

#[test]
fn known_sysvars_never_report_inert() {
    for (name, _) in canonical_entries() {
        let dna = LegacyDna::parse(&format!("start\n1 .{name} store\nstop")).unwrap();
        assert!(
            dna.compatibility_warnings().is_empty(),
            "known sysvar {name} warned: {:?}",
            dna.compatibility_warnings()
        );
    }

    let unknown = LegacyDna::parse("start\n1 .thisdoesnotexist store\nstop").unwrap();
    assert!(unknown.compatibility_warnings()[0].contains("unknown sysvar"));
    assert!(unknown.compatibility_warnings()[0].contains("address 0"));
}

#[test]
fn setboy_updates_persisted_buoyancy_readback_and_consumes_command() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let robot = engine
        .spawn(LegacyDna::parse("start\n16000 .setboy store\nend").unwrap())
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory_at(robot, 314).unwrap(), 0);
    assert_eq!(engine.memory_at(robot, 315).unwrap(), 16_000);
}

#[test]
fn body_transfer_sysvars_apply_once_with_exact_next_cycle_cost() {
    for (name, body_delta, energy_delta) in [("fdbody", -2, 20), ("strbody", 2, -20)] {
        let mut config = EngineConfig::testing();
        config.metabolism_cost = 0;
        config.vegetable_energy_per_tick = 0;
        config.sunlight_energy = 0;
        let mut engine = Engine::new(config).unwrap();
        let robot = engine
            .spawn(
                LegacyDna::parse(&format!("start\n20 .{name} store\nend")).unwrap(),
            )
            .unwrap();
        let before = engine.organism(robot).unwrap();
        let (before_body, before_energy) = (before.body, before.energy);

        engine.tick().unwrap();

        let after = engine.organism(robot).unwrap();
        assert_eq!(after.body, before_body + body_delta, "body delta for .{name}");
        assert_eq!(
            after.energy,
            before_energy + energy_delta,
            "energy delta for .{name}"
        );
        assert_eq!(engine.memory(robot, name).unwrap(), 0, "consumed .{name}");

        engine
            .replace_dna(robot, LegacyDna::parse("start\nend").unwrap())
            .unwrap();
        engine.tick().unwrap();
        let next = engine.organism(robot).unwrap();
        assert_eq!(next.body, after.body, "body repeated on next cycle for .{name}");
        assert_eq!(
            next.energy, after.energy,
            "energy repeated on next cycle for .{name}"
        );
    }
}

#[test]
fn capturing_commands_does_not_clear_their_registers() {
    let runtime = SysvarRuntime::new();
    let mut memory = VmMemory::default();
    memory.write_sysvar("up", 11);
    memory.write_sysvar("shoot", -6);
    memory.write_sysvar("shootval", 27);
    memory.write(50, 1_234);

    let movement = runtime.consume(CyclePhase::Movement, &mut memory).unwrap();
    let PhaseCommands::Movement(movement) = movement else {
        panic!("movement phase returned the wrong typed command DTO");
    };
    assert_eq!(movement.up, 11);
    assert_eq!(memory.read_sysvar("up"), 11);
    assert_eq!(memory.read_sysvar("shoot"), -6);

    let shot = runtime.consume(CyclePhase::Shot, &mut memory).unwrap();
    let PhaseCommands::Shot(shot) = shot else {
        panic!("shot phase returned the wrong typed command DTO");
    };
    assert_eq!(shot.kind, -6);
    assert_eq!(shot.value, 27);
    assert_eq!(memory.read_sysvar("shoot"), -6);
    assert_eq!(memory.read_sysvar("shootval"), 27);
    assert_eq!(memory.read(50), 1_234);

    assert!(
        runtime
            .definitions()
            .iter()
            .filter(|definition| definition.role == SysvarRole::Command)
            .all(|definition| definition.consume_at.is_some())
    );
}

#[test]
fn command_outcomes_control_when_and_how_registers_reset() {
    let runtime = SysvarRuntime::new();
    let mut memory = VmMemory::default();
    memory.write_sysvar("up", 11);
    memory.write_sysvar("fixang", 123);
    memory.write_sysvar("mkvirus", 13);
    memory.write_sysvar("delgene", 15);

    runtime.finish_command(1, CommandOutcome::Consumed, &mut memory);
    runtime.finish_command(468, CommandOutcome::Consumed, &mut memory);
    runtime.finish_command(335, CommandOutcome::Pending, &mut memory);
    runtime.finish_command(340, CommandOutcome::Excluded, &mut memory);

    assert_eq!(memory.read_sysvar("up"), 0);
    assert_eq!(memory.read_sysvar("fixang"), 32_000);
    assert_eq!(memory.read_sysvar("mkvirus"), 13);
    assert_eq!(memory.read_sysvar("delgene"), 15);
}

#[test]
fn only_legacy_touch_taste_and_edge_cells_clear_after_dna() {
    let runtime = SysvarRuntime::new();
    let mut memory = VmMemory::default();
    let transients: [u16; 12] = [201, 202, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214];
    for address in transients {
        memory.write(address.into(), address as i32);
    }
    memory.write(50, 8_765);

    runtime.clear_transients_after_dna(&mut memory);

    for address in transients {
        assert_eq!(memory.read(address.into()), 0, "address {address} survived");
        assert_eq!(
            runtime.definition(address).unwrap().clear_at,
            Some(CyclePhase::AfterDna)
        );
    }
    for address in [221, 437, 438, 449, 456, 464, 475, 478, 479] {
        memory.write(address, address);
        runtime.clear_transients_after_dna(&mut memory);
        assert_eq!(
            memory.read(address),
            address,
            "address {address} was cleared"
        );
    }
    assert_eq!(memory.read(50), 8_765);
}

#[test]
fn late_sensors_replace_prior_values_without_clearing_private_memory() {
    let runtime = SysvarRuntime::new();
    let mut memory = VmMemory::default();
    memory.write_sysvar("eye5", 88);
    memory.write_sysvar("refnrg", 77);
    memory.write(50, 4_321);

    runtime.publish(
        PhaseReadbacks::Vision(VisionReadbacks {
            values: vec![(505, 42), (709, 900)],
        }),
        &mut memory,
    );

    assert_eq!(memory.read_sysvar("eye5"), 42);
    assert_eq!(memory.read_sysvar("refnrg"), 900);
    assert_eq!(memory.read(50), 4_321);
}

#[test]
fn future_mechanism_commands_are_typed_and_phase_routed() {
    let runtime = SysvarRuntime::new();
    let mut memory = VmMemory::default();
    memory.write_sysvar("tie", 4);
    memory.write_sysvar("stifftie", 55);
    memory.write_sysvar("tienum", 3);
    memory.write_sysvar("mkvirus", 13);
    memory.write_sysvar("vshoot", 14);
    memory.write_sysvar("delgene", 15);

    let tie = runtime.consume(CyclePhase::Tie, &mut memory).unwrap();
    let PhaseCommands::Tie(TieCommands {
        create,
        stiffness,
        number,
        ..
    }) = tie
    else {
        panic!("tie phase returned the wrong typed command DTO");
    };
    assert_eq!((create, stiffness, number), (4, 55, 3));
    assert_eq!(memory.read_sysvar("tie"), 4);
    assert_eq!(memory.read_sysvar("stifftie"), 55);
    assert_eq!(memory.read_sysvar("tienum"), 3);
    assert_eq!(memory.read_sysvar("mkvirus"), 13);

    let virus = runtime.consume(CyclePhase::Virus, &mut memory).unwrap();
    let PhaseCommands::Virus(VirusCommands {
        make,
        shoot,
        delete_gene,
    }) = virus
    else {
        panic!("virus phase returned the wrong typed command DTO");
    };
    assert_eq!((make, shoot, delete_gene), (13, 14, 15));
    assert_eq!(memory.read_sysvar("mkvirus"), 13);
    assert_eq!(memory.read_sysvar("vshoot"), 14);
    assert_eq!(memory.read_sysvar("delgene"), 15);
}

fn testing_engine() -> Engine {
    Engine::new(EngineConfig::testing()).unwrap()
}

#[test]
fn engine_tick_resets_fixang_to_legacy_sentinel() {
    let mut engine = testing_engine();
    let robot = engine
        .spawn(LegacyDna::parse("start\n123 .fixang store\nstop").unwrap())
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(robot, "fixang").unwrap(), 32_000);
}

#[test]
fn engine_tick_setaim_overrides_relative_turns_and_mirrors_next_cycle_memory() {
    let mut engine = testing_engine();
    let robot = engine
        .spawn(
            LegacyDna::parse("start\n200 .aimsx store\n50 .aimdx store\n900 .setaim store\nstop")
                .unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.memory(robot, "aim").unwrap(), 900);
    assert_eq!(engine.memory(robot, "setaim").unwrap(), 900);

    engine
        .replace_dna(
            robot,
            LegacyDna::parse("start\n40 .aimsx store\n10 .aimdx store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.memory(robot, "aim").unwrap(), 930);
    assert_eq!(engine.memory(robot, "setaim").unwrap(), 930);
}

#[test]
fn engine_tick_clears_only_legacy_after_dna_transients_and_keeps_hitang() {
    let mut engine = testing_engine();
    let mut source = String::from("start\n777 .hitang store\n");
    for address in [201, 202, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214] {
        source.push_str(&format!("{address} {address} store\n"));
    }
    source.push_str("stop");
    let robot = engine.spawn(LegacyDna::parse(&source).unwrap()).unwrap();

    engine.tick().unwrap();

    for address in [201, 202, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214] {
        assert_eq!(
            engine.memory_at(robot, address).unwrap(),
            0,
            "address {address}"
        );
    }
    assert_eq!(engine.memory(robot, "hitang").unwrap(), 777);
}

#[test]
fn engine_tick_preserves_blocked_reproduction_until_birth_succeeds() {
    let mut engine = testing_engine();
    engine.update_legacy_restrictions(true, false);
    let robot = engine
        .spawn(
            LegacyDna::parse("cond\n*.50 0 =\nstart\n50 .repro store\n1 50 store\nstop\nend")
                .unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();
    assert_eq!(engine.population(), 1);
    assert_eq!(engine.memory(robot, "repro").unwrap(), 50);

    engine.update_legacy_restrictions(true, true);
    engine.tick().unwrap();
    assert_eq!(engine.population(), 2);
    assert_eq!(engine.memory(robot, "repro").unwrap(), 0);
}

#[test]
fn engine_tick_preserves_unfinished_virus_commands() {
    let mut engine = testing_engine();
    let robot = engine
        .spawn(
            LegacyDna::parse(
                "cond\n*.50 0 =\nstart\n12 .mkvirus store\n30 .vshoot store\n5 .delgene store\n1 50 store\nstop\nend",
            )
            .unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(robot, "mkvirus").unwrap(), 12);
    assert_eq!(engine.memory(robot, "vshoot").unwrap(), 30);
    assert_eq!(engine.memory(robot, "delgene").unwrap(), 5);
}

#[test]
fn engine_tick_keeps_aimshoot_when_shoot_kind_has_no_handler() {
    let mut engine = testing_engine();
    let robot = engine
        .spawn(
            LegacyDna::parse("start\n-5 .shoot store\n456 .aimshoot store\nstop").unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(robot, "shoot").unwrap(), 0);
    assert_eq!(engine.memory(robot, "shootval").unwrap(), 0);
    assert_eq!(engine.memory(robot, "aimshoot").unwrap(), 456);
}

#[test]
fn engine_tick_executes_delgene_in_the_authoritative_virus_phase() {
    let mut engine = testing_engine();
    let robot = engine
        .spawn(
            LegacyDna::parse(
                "start\n1 .delgene store\nstop\nstart\n99 .up store\nstop\nend",
            )
            .unwrap(),
        )
        .unwrap();
    let before = engine.dna(robot).unwrap().blocks().len();

    engine.tick().unwrap();

    assert!(engine.dna(robot).unwrap().blocks().len() < before);
    assert_eq!(engine.memory(robot, "delgene").unwrap(), 0);
}

#[test]
fn engine_tick_keeps_tie_port_selectors_distinct_from_tie_count() {
    let mut engine = testing_engine();
    let first = engine
        .spawn_at(
            LegacyDna::parse("start\n41 .tie store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let _second = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [400.0, 100.0])
        .unwrap();
    let third = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [800.0, 100.0])
        .unwrap();
    engine.tick().unwrap();

    engine
        .replace_dna(
            first,
            LegacyDna::parse("start\n77 .tie store\n41 .readtie store\nstop").unwrap(),
        )
        .unwrap();
    engine.move_organism(third, [100.0, 350.0]).unwrap();
    engine.tick().unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.memory(first, "numties").unwrap(), 2);
    assert_eq!(engine.memory(first, "tiepres").unwrap(), 77);
    assert_eq!(engine.memory(first, "tienum").unwrap(), 0);
    assert_eq!(engine.memory(first, "readtie").unwrap(), 41);
}

#[test]
fn engine_tick_preserves_tied_references_only_for_a_valid_selected_port() {
    let mut engine = testing_engine();
    let first = engine
        .spawn_at(
            LegacyDna::parse("start\n41 .tie store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let _second = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [110.0, 100.0])
        .unwrap();
    engine.tick().unwrap();

    engine
        .replace_dna(
            first,
            LegacyDna::parse("start\n777 .trefbody store\n41 .readtie store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(engine.memory(first, "trefbody").unwrap(), 777);

    engine
        .replace_dna(
            first,
            LegacyDna::parse("start\n999 .readtie store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();
    for address in [
        437, 438, 439, 440, 441, 442, 443, 444, 445, 446, 447, 448, 449, 456, 457, 458, 459, 460,
        461, 462, 463, 464, 465, 475, 478, 479,
    ] {
        assert_eq!(
            engine.memory_at(first, address).unwrap(),
            0,
            "address {address}"
        );
    }
}

fn engine_with_two_source_ties() -> (Engine, OrganismId, OrganismId, OrganismId) {
    let mut engine = testing_engine();
    let source = engine
        .spawn_at(
            LegacyDna::parse("start\n41 .tie store\nstop").unwrap(),
            [100.0, 100.0],
        )
        .unwrap();
    let port_41_partner = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [400.0, 100.0])
        .unwrap();
    let port_77_partner = engine
        .spawn_at(LegacyDna::parse("start\nstop").unwrap(), [800.0, 100.0])
        .unwrap();
    engine.tick().unwrap();

    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n77 .tie store\nstop").unwrap(),
        )
        .unwrap();
    engine
        .move_organism(port_77_partner, [100.0, 350.0])
        .unwrap();
    engine.tick().unwrap();
    assert_eq!(
        engine.snapshot().ties.len(),
        2,
        "fixture did not create both selected-port ties"
    );

    (engine, source, port_41_partner, port_77_partner)
}

#[test]
fn engine_tick_routes_tie_memory_only_through_the_selected_local_port() {
    let (mut engine, source, port_41_partner, port_77_partner) = engine_with_two_source_ties();
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n77 .tienum store\n50 .tieloc store\n1234 .tieval store\nstop")
                .unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory_at(port_41_partner, 50).unwrap(), 0);
    assert_eq!(engine.memory_at(port_77_partner, 50).unwrap(), 1_234);
}

#[test]
fn engine_tick_marks_tie_information_transfer_on_selected_endpoint() {
    let (mut engine, source, _, _) = engine_with_two_source_ties();
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n77 .tienum store\n50 .tieloc store\n1234 .tieval store\nstop")
                .unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();

    let marker = engine
        .robot_state(source)
        .unwrap()
        .ties
        .iter()
        .flatten()
        .find(|tie| tie.port == 77)
        .map(|tie| tie.information);
    assert_eq!(marker, Some(1));
}

#[test]
fn engine_tick_keeps_tie_memory_pending_without_a_valid_selected_port() {
    let (mut engine, source, port_41_partner, port_77_partner) = engine_with_two_source_ties();
    engine
        .replace_dna(
            source,
            LegacyDna::parse(
                "start\n999 .tienum store\n60 .tieloc store\n2001 .tieval store\nstop",
            )
            .unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory(source, "tieloc").unwrap(), 60);
    assert_eq!(engine.memory(source, "tieval").unwrap(), 2_001);
    assert_eq!(engine.memory_at(port_41_partner, 60).unwrap(), 0);
    assert_eq!(engine.memory_at(port_77_partner, 60).unwrap(), 0);

    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n61 .tieloc store\n2002 .tieval store\nstop").unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.memory(source, "tieloc").unwrap(), 61);
    assert_eq!(engine.memory(source, "tieval").unwrap(), 2_002);
    assert_eq!(engine.memory_at(port_41_partner, 61).unwrap(), 0);
    assert_eq!(engine.memory_at(port_77_partner, 61).unwrap(), 0);

    engine
        .replace_dna(
            source,
            LegacyDna::parse(
                "start\n41 .tienum store\n1001 .tieloc store\n2003 .tieval store\nstop",
            )
            .unwrap(),
        )
        .unwrap();
    engine.tick().unwrap();

    assert_eq!(engine.memory(source, "tieloc").unwrap(), 1_001);
    assert_eq!(engine.memory(source, "tieval").unwrap(), 2_003);
}

#[test]
fn engine_tick_resets_tie_memory_only_after_a_valid_selected_write() {
    let (mut engine, source, _, port_77_partner) = engine_with_two_source_ties();
    engine
        .replace_dna(
            source,
            LegacyDna::parse("start\n77 .tienum store\n62 .tieloc store\n2004 .tieval store\nstop")
                .unwrap(),
        )
        .unwrap();

    engine.tick().unwrap();

    assert_eq!(engine.memory_at(port_77_partner, 62).unwrap(), 2_004);
    assert_eq!(engine.memory(source, "tieloc").unwrap(), 0);
    assert_eq!(engine.memory(source, "tieval").unwrap(), 0);
    assert_eq!(engine.memory(source, "tienum").unwrap(), 0);
}
