use darwinbots_engine::{
    Engine, EngineConfig, LegacyDna, OrganismId, SaveFile, SpeciesDefinition, VisualPhenotype,
    generated_skin,
};

fn phenotype_engine(auto_speciation: bool, threshold: f32) -> Engine {
    Engine::new(EngineConfig {
        metabolism_cost: 0,
        auto_speciation,
        speciation_genetic_distance_percent: threshold,
        ..EngineConfig::testing()
    })
    .unwrap()
}

fn spawn_mutating_parent(engine: &mut Engine, vegetable: bool) -> OrganismId {
    let mut profile = darwinbots_engine::StandardMutationProfile::default();
    profile.point.one_in = 0.0;
    profile.delta.one_in = 0.0;
    profile.copy_error.one_in = 1.0;
    profile.copy_error.mean = 1.0;
    profile.copy_error.std_dev = 0.0;
    profile.insertion.one_in = 0.0;
    profile.reversal.one_in = 0.0;
    profile.minor_deletion.one_in = 0.0;
    profile.major_deletion.one_in = 0.0;
    let species = engine.register_species(SpeciesDefinition {
        name: if vegetable {
            "Visual Alga"
        } else {
            "Visual Animal"
        }
        .to_owned(),
        vegetable,
        color: if vegetable { 0xff4c_963b } else { 0xff23_9ac0 },
        mutation_profile: Some(profile),
        ..SpeciesDefinition::default()
    });
    engine
        .spawn_species_batch(
            LegacyDna::parse("start\n10 .up store\n50 .repro store\nstop").unwrap(),
            species,
            [[500.0, 500.0]],
            1_000,
        )
        .unwrap()[0]
}

#[test]
fn generated_skin_is_stable_and_inside_the_bot() {
    let first = generated_skin("Animal Minimalis", 7);
    let second = generated_skin("Animal Minimalis", 7);

    assert_eq!(first, second);
    assert!(
        first
            .iter()
            .all(|point| (0.15..=0.82).contains(&point.radius))
    );
    assert!(first.iter().all(|point| (0..1257).contains(&point.angle)));
}

#[test]
fn real_mutation_drifts_color_but_zero_changes_do_not() {
    let mut phenotype = VisualPhenotype::new(3, 0xff23_9ac0, generated_skin("Animal", 3));
    let original = phenotype.clone();
    let mut random = 91;

    phenotype.apply_color_mutation(0, false, &mut random);
    assert_eq!(phenotype, original);

    phenotype.apply_color_mutation(1, false, &mut random);
    assert_ne!(phenotype.color, original.color);
}

#[test]
fn autotroph_mutations_remain_green() {
    let mut phenotype = VisualPhenotype::new(4, 0xff4c_963b, generated_skin("Alga", 4));
    let mut random = 44;

    for _ in 0..200 {
        phenotype.apply_color_mutation(1, true, &mut random);
    }

    let [red, green, blue] = phenotype.rgb();
    assert!(green >= red && green >= blue);
}

#[test]
fn speciation_changes_one_skin_point_and_resets_distance() {
    let mut phenotype = VisualPhenotype::new(9, 0xff23_9ac0, generated_skin("Animal", 9));
    phenotype.accumulated_mutations = 12;
    let before = phenotype.skin;
    let mut random = 17;

    phenotype.apply_speciation(10, &mut random);

    assert_eq!(phenotype.lineage_id, 10);
    assert_eq!(phenotype.accumulated_mutations, 0);
    assert_eq!(
        before
            .iter()
            .zip(phenotype.skin)
            .filter(|(left, right)| left != &right)
            .count(),
        1
    );
}

#[test]
fn child_inherits_phenotype_then_mutation_drifts_only_the_child() {
    let mut engine = phenotype_engine(false, 25.0);
    let parent = spawn_mutating_parent(&mut engine, false);
    let parent_before = engine.organism(parent).unwrap().phenotype;

    engine.tick().unwrap();

    let parent_after = engine.organism(parent).unwrap();
    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.id != parent)
        .unwrap();
    assert_eq!(parent_after.phenotype, parent_before);
    assert_eq!(child.phenotype.skin, parent_before.skin);
    assert_eq!(child.phenotype.lineage_id, parent_before.lineage_id);
    assert_ne!(child.phenotype.color, parent_before.color);
}

#[test]
fn native_speciation_preserves_the_inherited_skin_and_resets_the_counter() {
    let mut engine = phenotype_engine(true, 1.0);
    let parent = spawn_mutating_parent(&mut engine, false);
    let parent_skin = engine.organism(parent).unwrap().phenotype.skin;

    engine.tick().unwrap();

    let child = engine
        .snapshot()
        .organisms
        .iter()
        .find(|organism| organism.id != parent)
        .unwrap();
    assert_ne!(child.phenotype.lineage_id, 0);
    assert_eq!(child.phenotype.skin, parent_skin);
    assert_eq!(child.phenotype.accumulated_mutations, 0);
}

#[test]
fn phenotype_round_trips_through_version_four_save() {
    let mut engine = phenotype_engine(true, 1.0);
    spawn_mutating_parent(&mut engine, true);
    engine.tick().unwrap();

    let bytes = SaveFile::encode(&engine).unwrap();
    assert_eq!(&bytes[..6], &[b'D', b'B', b'3', b'S', 4, 0]);
    let restored = SaveFile::decode(&bytes).unwrap();

    // Inspector readbacks are rebuilt after loading; this test qualifies the
    // persisted phenotype of every parent and child.
    let phenotypes = |engine: &Engine| {
        engine
            .snapshot()
            .organisms
            .iter()
            .map(|organism| (organism.id, organism.phenotype.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(phenotypes(&restored), phenotypes(&engine));
}

#[test]
fn zero_native_speciation_threshold_survives_updates_and_save_resume() {
    // DB2 permits zero: any accumulated mutation can cross the threshold.
    let mut engine = phenotype_engine(false, 0.0);
    engine
        .update_speciation_settings(Some(true), Some(0.0))
        .unwrap();
    let bytes = SaveFile::encode(&engine).unwrap();
    let mut restored = SaveFile::decode(&bytes).unwrap();
    restored
        .update_speciation_settings(None, Some(0.0))
        .unwrap();
    assert_eq!(SaveFile::encode(&restored).unwrap(), bytes);
    for invalid in [-1.0, f32::NAN, f32::INFINITY] {
        assert!(
            restored
                .update_speciation_settings(Some(false), Some(invalid))
                .is_err()
        );
        assert_eq!(SaveFile::encode(&restored).unwrap(), bytes);
    }
}
