//! NeoMutations.bas 138 and Robots.bas 2394/2828: powered mutation dose.
use super::*;

impl Engine {
    pub(super) fn native_inherit_epigenetic_dose(&mut self, child: OrganismId, parent_dose: f64) {
        let settings = self.config.advanced_mutation.epigenetic;
        if !settings.reset_enabled { return; }
        let Ok(robot) = self.slots.get_mut(child) else { return };
        robot.heredity.native_mutation_dose = parent_dose
            + (robot.program.mutation_metadata.native_last_mutations as f64).powf(f64::from(settings.native_exponent));
        // A previously unexposed parent cannot cause a reset on its first birth.
        if robot.heredity.native_mutation_dose > settings.overload_threshold as f64 && parent_dose > 0.0 {
            robot.heredity.native_mutation_dose = 0.0;
            for address in 971..=975 { robot.program.memory.write(address, 0); }
            robot.heredity.epigenetic_staging.fill(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dose_is_inherited_but_reset_requires_positive_parent_dose_and_strict_threshold() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world.spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2]).unwrap();
        world.config.advanced_mutation.epigenetic.reset_enabled = true;
        world.config.advanced_mutation.epigenetic.native_exponent = 1.0;
        world.config.advanced_mutation.epigenetic.overload_threshold = 17;
        let robot = world.slots.get_mut(id).unwrap();
        robot.program.mutation_metadata.native_last_mutations = 18;
        robot.program.memory.write(971, 42);
        robot.heredity.epigenetic_staging.fill(7);
        world.native_inherit_epigenetic_dose(id, 0.0);
        assert_eq!(world.slots.get(id).unwrap().program.memory.read(971), 42);
        world.slots.get_mut(id).unwrap().program.mutation_metadata.native_last_mutations = 16;
        world.native_inherit_epigenetic_dose(id, 1.0);
        assert_eq!(world.slots.get(id).unwrap().heredity.native_mutation_dose, 17.0);
        world.native_inherit_epigenetic_dose(id, 2.0);
        assert_eq!(world.slots.get(id).unwrap().heredity.native_mutation_dose, 0.0);
        assert_eq!(world.slots.get(id).unwrap().program.memory.read(971), 0);
        assert_eq!(world.slots.get(id).unwrap().heredity.epigenetic_staging, [0; 15]);
    }

    #[test]
    fn lifelong_dose_accumulates_new_mutations_without_clearing_inherited_memory() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world.spawn_at(LegacyDna::parse("start 12 stop").unwrap(), [500.0; 2]).unwrap();
        world.config.advanced_mutation.epigenetic.reset_enabled = true;
        world.config.advanced_mutation.epigenetic.overload_threshold = 1;
        let robot = world.slots.get_mut(id).unwrap();
        robot.counters.age = 1;
        robot.heredity.native_mutation_dose = 100.0;
        robot.heredity.epigenetic_staging.fill(9);
        robot.program.mutation_metadata.total_mutations = 100;
        robot.program.mutation_profile.delta.one_in = 0.0;
        robot.program.mutation_profile.point = crate::MutationRate { one_in: 1_000_000.0, mean: 1.0, std_dev: 0.0 };
        robot.program.point_mutation_schedule = Some(crate::PointMutationSchedule { location: 2, cycle: 1 });
        let settings = world.config.advanced_mutation.clone();
        world.apply_mutation_to_organism(id, &settings, 1.0, MutationContext::Lifelong, None);
        let robot = world.slots.get(id).unwrap();
        let changes = robot.program.mutation_metadata.total_mutations - 100;
        assert!(changes > 0);
        assert_eq!(robot.heredity.native_mutation_dose,
            100.0 + (changes as f64).powf(f64::from(settings.epigenetic.native_exponent)));
        assert_eq!(robot.heredity.epigenetic_staging, [9; 15]);
        assert_eq!(robot.program.mutation_metadata.mutation_class_counts[&StandardMutationKind::Point], changes);
    }

    #[test]
    fn manual_native_child_keeps_mothers_move_flag_profile_and_ordered_definition_reset() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let mother = world.spawn_at(LegacyDna::parse("def inherited 971\nstart stop").unwrap(), [500.0; 2]).unwrap();
        let father = world.spawn_at(LegacyDna::parse("NewMove\nstart stop").unwrap(), [900.0; 2]).unwrap();
        world.slots.get_mut(mother).unwrap().program.mutation_profile.copy_error.one_in = 123.0;
        world.slots.get_mut(mother).unwrap().program.memory.write(971, 17);
        let child = world.manual_reproduce(mother, Some(father), [700.0; 2]).unwrap();
        let robot = world.slots.get(child).unwrap();
        assert!(!robot.flags.new_move);
        assert_eq!(robot.program.mutation_profile.copy_error.one_in, 123.0);
        assert_eq!(robot.program.memory.read(971), 17);
        assert_eq!(robot.program.dna.native_user_variables().len(), 0);
    }

    #[test]
    fn modern_save_preserves_fractional_dose_and_exported_epigene_does_not_edit_live_dna() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world.spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2]).unwrap();
        world.slots.get_mut(id).unwrap().heredity.native_mutation_dose = 3.75;
        world.slots.get_mut(id).unwrap().program.memory.write(971, 9);
        world.slots.get_mut(id).unwrap().program.memory.write(990, -12);
        world.config.advanced_mutation.epigenetic.save_as_temporary_gene = true;
        let original = world.slots.get(id).unwrap().program.dna.clone();
        let exported = world.export_dna(id).unwrap();
        let expected = LegacyDna::parse("start 9 971 store -12 990 store *.thisgene .delgene store stop start stop").unwrap();
        assert_eq!(LegacyDna::parse(&exported).unwrap().blocks(), expected.blocks());
        assert_eq!(world.slots.get(id).unwrap().program.dna.blocks(), original.blocks());
        let bytes = crate::SaveFile::encode(&world).unwrap();
        let loaded = crate::SaveFile::decode(&bytes).unwrap();
        assert_eq!(loaded.slots.get(id).unwrap().heredity.native_mutation_dose, 3.75);
    }
}
