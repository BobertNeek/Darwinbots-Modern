//! Manual insertion reuses the selected species and the clicked position.

use super::*;

impl Engine {
    pub(crate) fn insert_registered_species_at(
        &mut self,
        name: &str,
        dna: LegacyDna,
        position: [f32; 2],
        initial_energy: i32,
    ) -> Result<(SpeciesId, OrganismId), EngineError> {
        if position.iter().any(|value| !value.is_finite()) || initial_energy < 1 {
            return Err(EngineError::Invariant(
                "species insertion requires finite coordinates and positive energy".into(),
            ));
        }
        // main.frm resolves Combo1 against the existing species list, then
        // Globals.aggiungirob adds one robot at x/y. Startup batch size and
        // placement bounds do not apply to this player action.
        let index = self
            .species
            .iter()
            .position(|species| species.name == name)
            .ok_or_else(|| EngineError::Invariant(format!("species '{name}' does not exist")))?;
        let species = SpeciesId(index as u32);
        let id = self.spawn_native_script_robot(dna, species, position)?;
        let robot = self.slots.get_mut(id)?;
        robot.materials.energy = initial_energy as f32;
        robot.materials.synchronize_energy();
        if self.native_random.is_none() {
            robot.program.memory.write(MEM_NRG, initial_energy);
        }
        if self.native_random.is_some() {
            self.native_buckets.synchronize(
                [self.config.world_width, self.config.world_height],
                &self.slots,
            );
        }
        // Keep the startup template's population, energy and genome unchanged.
        self.publish_snapshot();
        Ok((species, id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SaveFile;

    fn world() -> (Engine, SpeciesId, LegacyDna) {
        let mut engine = Engine::new(EngineConfig {
            organism_capacity: 4,
            ..EngineConfig::testing()
        })
        .unwrap();
        let species = engine.register_species(SpeciesDefinition {
            name: "Existing player species".into(),
            minimum_population: 3,
            ..SpeciesDefinition::default()
        });
        let dna = LegacyDna::parse("start 1 .up store stop").unwrap();
        engine
            .spawn_species_batch(dna.clone(), species, [[100.0, 200.0]; 3], 1_000)
            .unwrap();
        (engine, species, dna)
    }

    #[test]
    fn insertion_reuses_species_without_replacing_the_startup_template() {
        let (mut engine, species, dna) = world();
        let template = serde_json::to_value(&engine.species_templates).unwrap();
        let definitions = serde_json::to_value(&engine.species).unwrap();
        let (inserted_species, id) = engine
            .insert_registered_species_at("Existing player species", dna, [321.0, 654.0], 2_000)
            .unwrap();
        assert_eq!(inserted_species, species);
        assert_eq!(engine.snapshot().organisms.len(), 4);
        let robot = engine.slots.get(id).unwrap();
        assert_eq!(robot.identity.species, species);
        assert_eq!(robot.kinematics.position, [321.0, 654.0]);
        assert_eq!(robot.materials.energy, 2_000.0);
        assert_eq!(serde_json::to_value(&engine.species).unwrap(), definitions);
        assert_eq!(
            serde_json::to_value(&engine.species_templates).unwrap(),
            template
        );
    }

    #[test]
    fn native_insertion_keeps_species_identity_and_exact_player_coordinates() {
        let mut engine = Engine::from_native_sim(include_bytes!(
            "../../tests/fixtures/db24832-original-user.sim"
        ))
        .unwrap();
        assert!(engine.native_random.is_some());
        for index in 0..engine.species.len() {
            let species = SpeciesId(index as u32);
            let name = engine.species[index].name.clone();
            let dna = engine
                .slots
                .iter()
                .filter_map(|slot| slot.organism.as_ref())
                .find(|robot| robot.identity.species == species)
                .unwrap()
                .program
                .dna
                .clone();
            let population = engine.snapshot().organisms.len();
            let tick = engine.snapshot().tick;
            let template = serde_json::to_value(&engine.species_templates).unwrap();
            let (inserted_species, id) = engine
                .insert_registered_species_at(&name, dna, [456.0, 789.0], 1_234)
                .unwrap();
            assert_eq!(inserted_species, species);
            assert_eq!(engine.species.len(), 2);
            assert_eq!(engine.snapshot().organisms.len(), population + 1);
            assert_eq!(engine.snapshot().tick, tick);
            let robot = engine.slots.get(id).unwrap();
            assert_eq!(robot.kinematics.position, [456.0, 789.0]);
            assert_eq!(robot.materials.energy, 1_234.0);
            assert_eq!(
                serde_json::to_value(&engine.species_templates).unwrap(),
                template
            );
        }
    }

    #[test]
    fn invalid_or_full_insertion_preserves_complete_saved_state() {
        let (mut engine, _, dna) = world();
        engine
            .insert_registered_species_at(
                "Existing player species",
                dna.clone(),
                [300.0, 400.0],
                1_000,
            )
            .unwrap();
        let before = SaveFile::encode(&engine).unwrap();
        for (name, position, energy) in [
            ("missing", [1.0, 2.0], 1_000),
            ("Existing player species", [f32::NAN, 2.0], 1_000),
            ("Existing player species", [1.0, 2.0], 0),
            ("Existing player species", [1.0, 2.0], 1_000),
        ] {
            assert!(
                engine
                    .insert_registered_species_at(name, dna.clone(), position, energy)
                    .is_err()
            );
            assert_eq!(SaveFile::encode(&engine).unwrap(), before);
        }
    }
}
