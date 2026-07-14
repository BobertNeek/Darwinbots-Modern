use darwinbots_engine::{VisualPhenotype, generated_skin};

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
    let mut phenotype =
        VisualPhenotype::new(3, 0xff23_9ac0, generated_skin("Animal", 3));
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
    let mut phenotype =
        VisualPhenotype::new(9, 0xff23_9ac0, generated_skin("Animal", 9));
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
