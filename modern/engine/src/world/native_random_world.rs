#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::native_random::NativeRandom;

    #[test]
    fn dna_random_draws_share_one_native_stream_in_slot_order() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let dna = LegacyDna::parse("start 100 rnd 900 store stop").unwrap();
        let first = world.spawn_at(dna.clone(), [100.0; 2]).unwrap();
        let second = world.spawn_at(dna, [700.0; 2]).unwrap();
        let mut expected = NativeRandom::with_user_seed(world.config.seed);
        let first_value = expected.integer(0, 100);
        let second_value = expected.integer(0, 100);
        world.execute_dna_phase(&mut CycleRecorder::disabled(1)).unwrap();
        assert_eq!(world.memory_at(first, 900).unwrap(), first_value);
        assert_eq!(world.memory_at(second, 900).unwrap(), second_value);
        assert_eq!(world.native_random.as_ref().unwrap().state(), expected.state());
    }

    #[test]
    fn save_resume_preserves_the_global_stream_across_later_dna_draws() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world.spawn_at(LegacyDna::parse("start 32000 rnd 900 store stop").unwrap(), [500.0; 2]).unwrap();
        world.execute_dna_phase(&mut CycleRecorder::disabled(1)).unwrap();
        let mut resumed = crate::SaveFile::decode(&crate::SaveFile::encode(&world).unwrap()).unwrap();
        for cycle in 2..12 {
            world.execute_dna_phase(&mut CycleRecorder::disabled(cycle)).unwrap();
            resumed.execute_dna_phase(&mut CycleRecorder::disabled(cycle)).unwrap();
            assert_eq!(world.memory_at(id, 900).unwrap(), resumed.memory_at(id, 900).unwrap());
            assert_eq!(world.native_random, resumed.native_random);
        }
    }
}
