use darwinbots_engine::{
    Engine, MutationRate, SaveFile, StandardMutationKind, StandardMutationProfile,
};

const ORIGINAL_WORLD: &[u8] = include_bytes!("fixtures/db24832-original-user.sim");

fn rate(one_in: f64, mean: f64, std_dev: f64) -> MutationRate {
    MutationRate {
        one_in,
        mean,
        std_dev,
    }
}

#[test]
fn mutation_enabled_original_world_preserves_complete_state_across_repeated_saves() {
    let mut continuous = Engine::from_native_sim(ORIGINAL_WORLD).unwrap();
    // Keep structural growth disabled while exercising native point clocks,
    // Gaussian draws, inherited copy errors and evolving mutation rates.
    let profile = StandardMutationProfile {
        point: rate(200.0, 3.0, 1.0),
        point2: rate(200.0, 3.0, 1.0),
        delta: rate(5.0, 3.0, 1.0),
        copy_error: rate(200.0, 1.0, 0.0),
        copy_error2: rate(0.0, 1.0, 0.0),
        insertion: rate(0.0, 1.0, 0.0),
        reversal: rate(0.0, 3.0, 1.0),
        minor_deletion: rate(0.0, 1.0, 0.0),
        major_deletion: rate(0.0, 3.0, 1.0),
        amplification: rate(0.0, 250.0, 75.0),
        translocation: rate(0.0, 250.0, 75.0),
        ..StandardMutationProfile::default()
    };
    continuous.set_default_mutation_profile(profile).unwrap();
    continuous.update_mutation_settings(true, 1.0, &[]).unwrap();
    // The genuine save also disables mutation in each robot and species.
    // Enable those fixture permissions explicitly, keeping every other saved
    // field intact. This is a configured Rust continuation, not an original
    // executable trajectory comparison.
    let saved = SaveFile::encode(&continuous).unwrap();
    let mut payload: serde_json::Value = serde_json::from_slice(&saved[10..]).unwrap();
    for species in payload["species"].as_array_mut().unwrap() {
        species["restrictions"]["disable_mutations"] = false.into();
    }
    for slot in payload["slots"].as_array_mut().unwrap() {
        if !slot["organism"].is_null() {
            slot["organism"]["flags"]["can_mutate"] = true.into();
        }
    }
    let json = serde_json::to_vec(&payload).unwrap();
    let mut configured = saved[..6].to_vec();
    configured.extend_from_slice(&(json.len() as u32).to_le_bytes());
    configured.extend_from_slice(&json);
    continuous = SaveFile::decode(&configured).unwrap();
    assert!(
        continuous
            .snapshot()
            .organisms
            .iter()
            .any(|robot| { continuous.robot_state(robot.id).unwrap().flags.can_mutate }),
        "the imported world must contain mutation-capable robots"
    );
    let initial_mutations = continuous.snapshot().stats.mutations;
    let mut resumed = SaveFile::decode(&SaveFile::encode(&continuous).unwrap()).unwrap();
    for checkpoint in 1..=4 {
        continuous.tick_many(250).unwrap();
        resumed.tick_many(250).unwrap();
        continuous.validate_invariants().unwrap();
        resumed.validate_invariants().unwrap();
        let uninterrupted = SaveFile::encode(&continuous).unwrap();
        let restarted = SaveFile::encode(&resumed).unwrap();
        assert!(
            uninterrupted == restarted,
            "mutation-enabled complete state differs at checkpoint {checkpoint}, cycle {}",
            continuous.snapshot().tick
        );
        println!(
            "native mutation CPU cycle={} robots={} births={} mutations={} save_bytes={}",
            continuous.snapshot().tick,
            continuous.snapshot().organisms.len(),
            continuous.snapshot().stats.births,
            continuous.snapshot().stats.mutations,
            uninterrupted.len()
        );
        resumed = SaveFile::decode(&restarted).unwrap();
    }
    assert!(
        continuous.snapshot().stats.mutations > initial_mutations,
        "this continuation check must actually exercise mutations"
    );
    for kind in [
        StandardMutationKind::Point,
        StandardMutationKind::Delta,
        StandardMutationKind::CopyError,
    ] {
        let count: u64 = continuous
            .snapshot()
            .organisms
            .iter()
            .map(|robot| robot.mutation_class_counts.get(&kind).copied().unwrap_or(0))
            .sum();
        assert!(
            count > 0,
            "the continuation must actually exercise {kind:?}"
        );
        println!("native mutation CPU surviving class={kind:?} count={count}");
    }
}
