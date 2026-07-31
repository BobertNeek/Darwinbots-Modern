use std::collections::{BTreeMap, BTreeSet};

use darwinbots_engine::{
    CyclePhase, LegacyDna, PhaseCommands, PhaseReadbacks, SysvarRole, SysvarRuntime, TieCommands,
    VirusCommands, VisionReadbacks, VmMemory, sysvar_address,
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
fn commands_clear_at_their_declared_consumption_phase() {
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
    assert_eq!(memory.read_sysvar("up"), 0);
    assert_eq!(memory.read_sysvar("shoot"), -6);

    let shot = runtime.consume(CyclePhase::Shot, &mut memory).unwrap();
    let PhaseCommands::Shot(shot) = shot else {
        panic!("shot phase returned the wrong typed command DTO");
    };
    assert_eq!(shot.kind, -6);
    assert_eq!(shot.value, 27);
    assert_eq!(memory.read_sysvar("shoot"), 0);
    assert_eq!(memory.read_sysvar("shootval"), 0);
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
fn touch_taste_edge_and_tie_references_clear_after_dna() {
    let runtime = SysvarRuntime::new();
    let mut memory = VmMemory::default();
    let transients: [u16; 21] = [
        201, 202, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 221, 437, 438, 449, 456, 464,
        475, 478, 479,
    ];
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
    assert_eq!(memory.read_sysvar("tie"), 0);
    assert_eq!(memory.read_sysvar("stifftie"), 0);
    assert_eq!(memory.read_sysvar("tienum"), 0);
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
    assert_eq!(memory.read_sysvar("mkvirus"), 0);
    assert_eq!(memory.read_sysvar("vshoot"), 0);
    assert_eq!(memory.read_sysvar("delgene"), 0);
}
