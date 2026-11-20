use darwinbots_engine::{Engine, EngineConfig, LegacyDna, SpeciesDefinition, SpeciesId};

#[test]
fn live_menu_policies_publish_authoritative_state() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let species = engine.register_species(SpeciesDefinition {
        name: "Menu Test".to_owned(),
        mutation_rate: 1.0,
        color: 0xff239ac0,
        ..SpeciesDefinition::default()
    });
    let robot = engine
        .spawn_species_at(
            LegacyDna::parse("start\nstop").unwrap(),
            species,
            [100.0, 200.0],
        )
        .unwrap();

    engine
        .update_mutation_settings(false, 2.25, &[(species, 3.5)])
        .unwrap();
    engine.set_ties_enabled(false);
    engine.update_legacy_restrictions(false, false);
    engine.set_species_color(species, 0xffcc5522).unwrap();
    engine.set_organism_aim(robot, 314).unwrap();

    let snapshot = engine.snapshot();
    assert!(!snapshot.mutations_enabled);
    assert_eq!(snapshot.global_mutation_multiplier, 2.25);
    assert_eq!(snapshot.species[species.0 as usize].mutation_rate, 3.5);
    assert_eq!(snapshot.species[species.0 as usize].color, 0xffcc5522);
    assert!(!snapshot.ties_enabled);
    assert!(!snapshot.robot_fixing_allowed);
    assert!(!snapshot.asexual_reproduction_allowed);
    assert_eq!(snapshot.organisms[0].aim, 314);
}

#[test]
fn make_new_species_moves_only_the_selected_robot() {
    let mut engine = Engine::new(EngineConfig::testing()).unwrap();
    let source = engine.register_species(SpeciesDefinition {
        name: "Source".to_owned(),
        ..SpeciesDefinition::default()
    });
    let dna = LegacyDna::parse("start\nstop").unwrap();
    let first = engine
        .spawn_species_at(dna.clone(), source, [100.0, 100.0])
        .unwrap();
    let second = engine
        .spawn_species_at(dna, source, [200.0, 100.0])
        .unwrap();

    let descendant = engine
        .make_new_species(first, "Descendant".to_owned(), 0xffcc5522)
        .unwrap();

    assert_ne!(descendant, SpeciesId(source.0));
    let snapshot = engine.snapshot();
    let first_species = snapshot
        .organisms
        .iter()
        .find(|value| value.id == first)
        .unwrap()
        .species;
    let second_species = snapshot
        .organisms
        .iter()
        .find(|value| value.id == second)
        .unwrap()
        .species;
    assert_eq!(first_species, descendant);
    assert_eq!(second_species, source);
    assert_eq!(snapshot.species[descendant.0 as usize].name, "Descendant");
}
