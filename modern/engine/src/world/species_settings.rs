use super::*;
use crate::SpeciesSettings;

impl Engine {
    /// Edit future startup/repopulation policy. Existing robots keep their IDs,
    /// genomes, resources and inherited restrictions, as in OptionsForm Change.
    pub fn update_species_settings(
        &mut self,
        settings: Vec<SpeciesSettings>,
    ) -> Result<(), EngineError> {
        if settings.is_empty() || self.local_mode.config.is_some() {
            return Err(EngineError::Invariant(
                "Species settings require a nonempty roster outside a local competition.".into(),
            ));
        }
        let mut retained = std::collections::HashSet::new();
        let mut prepared = Vec::with_capacity(settings.len());
        for entry in settings {
            if let Some(id) = entry.species_id {
                if self.species.get(id.0 as usize).is_none() || !retained.insert(id) {
                    return Err(EngineError::Invariant(
                        "Species settings contain an invalid or duplicate identity.".into(),
                    ));
                }
            }
            if entry.definition.name.trim().is_empty()
                || entry.initial_energy <= 0
                || !entry.definition.mutation_rate.is_finite()
                || entry.definition.mutation_rate < 0.0
            {
                return Err(EngineError::Invariant("Invalid species settings.".into()));
            }
            entry
                .definition
                .validate_placement_bounds()
                .map_err(EngineError::Invariant)?;
            if let Some(profile) = &entry.definition.mutation_profile {
                profile.validate()?;
            }
            let dna = self.load_dna_source(&entry.dna)?;
            prepared.push((entry, dna));
        }
        // A removed startup species can still have living descendants. Retain
        // their registry identity and export source; stop future repopulation.
        for (index, definition) in self.species.iter_mut().enumerate().skip(1) {
            if !retained.contains(&SpeciesId(index as u32)) {
                definition.reseed = false;
                definition.minimum_population = 0;
                if let Some(template) = self.species_templates[index].as_mut() {
                    template.retired = true;
                    template.initial_population = 0;
                }
            }
        }
        for (entry, dna) in prepared {
            let mut definition = entry.definition;
            let id = if let Some(id) = entry.species_id {
                definition.lineage_id = self.species[id.0 as usize].lineage_id;
                self.species[id.0 as usize] = definition;
                id
            } else {
                self.register_species_unpublished(definition)
            };
            self.species_templates[id.0 as usize] = Some(SpeciesTemplate {
                retired: false,
                comment: entry.comment,
                dna,
                source_text: Some(entry.dna),
                initial_energy: entry.initial_energy,
                initial_population: entry.initial_count,
            });
        }
        self.publish_snapshot();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SaveFile;

    fn entry(id: Option<SpeciesId>, name: &str) -> SpeciesSettings {
        SpeciesSettings {
            species_id: id,
            definition: SpeciesDefinition {
                name: name.into(),
                minimum_population: 3,
                reseed: true,
                ..SpeciesDefinition::default()
            },
            dna: "start 1 .up store stop".into(),
            initial_energy: 777,
            initial_count: 4,
            comment: "Kept description".into(),
        }
    }

    #[test]
    fn roster_changes_preserve_living_robots_and_survive_save_resume() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let first = world.register_species(SpeciesDefinition {
            name: "First".into(),
            ..SpeciesDefinition::default()
        });
        let second = world.register_species(SpeciesDefinition {
            name: "Second".into(),
            ..SpeciesDefinition::default()
        });
        let id = world
            .spawn_species_batch(LegacyDna::parse("stop").unwrap(), first, [[500.0; 2]], 1000)
            .unwrap()[0];
        world
            .spawn_species_batch(LegacyDna::parse("stop").unwrap(), second, [[800.0; 2]], 900)
            .unwrap();
        let before = world.snapshot().organisms.clone();
        world
            .update_species_settings(vec![entry(Some(first), "Renamed"), entry(None, "Added")])
            .unwrap();
        assert_eq!(world.snapshot().organisms, before);
        assert_eq!(world.organism(id).unwrap().energy, 1000);
        assert_eq!(world.tick, 0);
        assert!(!world.species[second.0 as usize].reseed);
        let configured = world.configuration_view().unwrap();
        let configured = configured["species"].as_array().unwrap();
        assert!(configured.iter().all(|value| value["name"] != "Second"));
        assert!(configured.iter().any(|value| value["name"] == "Added"));
        let save = SaveFile::encode(&world).unwrap();
        let resumed = SaveFile::decode(&save).unwrap();
        assert_eq!(
            resumed.configuration_view().unwrap(),
            world.configuration_view().unwrap()
        );
        assert_eq!(SaveFile::encode(&resumed).unwrap(), save);
        assert!(
            resumed
                .export_species_template_dna(first)
                .unwrap()
                .contains(".up")
        );
    }

    #[test]
    fn invalid_roster_is_atomic() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world.register_species(SpeciesDefinition::default());
        let before = SaveFile::encode(&world).unwrap();
        let mut invalid = entry(None, "Invalid");
        invalid.definition.placement_bounds = [2.0; 4];
        assert!(
            world
                .update_species_settings(vec![entry(Some(id), "Changed"), invalid])
                .is_err()
        );
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
        assert!(
            world
                .update_species_settings(vec![entry(Some(id), "One"), entry(Some(id), "Two")])
                .is_err()
        );
        assert_eq!(SaveFile::encode(&world).unwrap(), before);
    }
}
