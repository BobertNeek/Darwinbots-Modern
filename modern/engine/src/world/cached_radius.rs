//! Radius is stored robot state. Death does not call FindRadius; body/decay does.
#[cfg(test)]
mod tests {
    use super::super::*;

    fn scene(backend: BackendPreference) -> (Engine, OrganismId) {
        let mut engine = Engine::new(EngineConfig {
            backend,
            world_width: 3000.0,
            world_height: 2000.0,
            metabolism_cost: 0,
            vegetable_energy_per_tick: 0,
            corpse_decay: CorpseDecaySettings {
                enabled: true,
                mode: crate::CorpseDecayMode::WasteShot,
                period: 100,
                packet_size: 10,
                ..Default::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let id = engine
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0, 500.0])
            .unwrap();
        (engine, id)
    }

    #[test]
    fn disabled_chloroplast_management_with_no_body_command_keeps_the_cache() {
        let (mut engine, id) = scene(BackendPreference::Cpu);
        let robot = engine.slots.get_mut(id).unwrap();
        robot.flags.no_chloroplasts = true;
        robot.materials.radius = 200.0;
        engine.tick().unwrap();
        assert_eq!(engine.slots.get(id).unwrap().materials.radius, 200.0);
    }

    #[test]
    fn spawned_snapshot_and_render_use_the_initialized_native_radius() {
        let (engine, id) = scene(BackendPreference::Cpu);
        let radius = crate::physics::organism_radius(1000.0, 0.0, false);
        assert_eq!(engine.slots.get(id).unwrap().materials.radius, radius);
        assert_eq!(engine.snapshot().organisms[0].radius, radius);
        assert_eq!(engine.snapshot().render_instances[0].radius, radius);
    }

    fn assert_death_retains_cached_radius(backend: BackendPreference) {
        let (mut engine, id) = scene(backend);
        assert_eq!(
            engine.capabilities.active,
            if backend == BackendPreference::Cpu {
                BackendKind::Cpu
            } else {
                BackendKind::Gpu
            }
        );
        assert!(engine.capabilities.fallback_reason.is_none());
        let robot = engine.slots.get_mut(id).unwrap();
        robot.materials.chloroplasts = 32_000.0;
        robot.materials.radius = 415.0;
        engine.become_native_corpse(id).unwrap();
        let other = engine
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [1000.0, 500.0])
            .unwrap();
        engine.tick().unwrap();
        let corpse = engine.slots.get(id).unwrap();
        assert_eq!(corpse.materials.chloroplasts, 0.0);
        assert_eq!(corpse.materials.radius, 415.0);
        assert!(corpse.kinematics.position[0] < 500.0);
        assert!(engine.slots.get(other).unwrap().kinematics.position[0] > 1000.0);
        let rendered = engine
            .snapshot()
            .render_instances
            .iter()
            .find(|instance| instance.slot == id.slot())
            .unwrap();
        assert_eq!(rendered.radius, 415.0);
        assert!(engine.capabilities.fallback_reason.is_none());
    }

    #[test]
    fn death_retains_the_large_chloroplast_radius_on_cpu() {
        assert_death_retains_cached_radius(BackendPreference::Cpu);
    }

    #[test]
    #[ignore = "requires a physical GPU; run with --ignored on the GPU qualification host"]
    fn death_retains_the_large_chloroplast_radius_on_physical_gpu() {
        assert_death_retains_cached_radius(BackendPreference::Gpu);
    }

    #[test]
    fn disabled_dna_keeps_the_cached_radius_instead_of_refreshing_it_during_physics() {
        let (mut engine, id) = scene(BackendPreference::Cpu);
        let robot = engine.slots.get_mut(id).unwrap();
        robot.flags.can_execute_dna = false;
        robot.materials.radius = 200.0;
        engine.tick().unwrap();
        assert_eq!(engine.slots.get(id).unwrap().materials.radius, 200.0);
        assert_eq!(engine.snapshot().render_instances[0].radius, 200.0);
    }

    #[test]
    fn corpse_radius_survives_modern_save_until_the_next_native_decay_event() {
        let (mut engine, id) = scene(BackendPreference::Cpu);
        engine.config.corpse_decay.period = 2;
        engine.slots.get_mut(id).unwrap().materials.radius = 415.0;
        engine.become_native_corpse(id).unwrap();
        engine.tick().unwrap();
        let mut resumed =
            crate::SaveFile::decode(&crate::SaveFile::encode(&engine).unwrap()).unwrap();
        assert_eq!(resumed.slots.get(id).unwrap().materials.radius, 415.0);
        resumed.tick().unwrap();
        let robot = resumed.slots.get(id).unwrap();
        assert_eq!(robot.materials.body, 999.0);
        assert_eq!(
            robot.materials.radius,
            crate::physics::organism_radius(999.0, 0.0, false)
        );
    }

    #[test]
    fn native_loader_calculates_radius_before_the_saved_chloroplast_field() {
        let (mut engine, id) = scene(BackendPreference::Cpu);
        engine.slots.get_mut(id).unwrap().materials.chloroplasts = 32_000.0;
        let loaded = Engine::from_native_sim(&engine.export_native_sim().unwrap()).unwrap();
        assert_eq!(
            loaded.slots[0].organism.as_ref().unwrap().materials.radius,
            crate::physics::organism_radius(1000.0, 0.0, false)
        );
        assert_eq!(
            loaded.slots[0]
                .organism
                .as_ref()
                .unwrap()
                .materials
                .chloroplasts,
            32_000.0
        );
    }

    #[test]
    fn old_save_without_radius_reconstructs_it_while_present_cached_values_are_preserved() {
        let (engine, id) = scene(BackendPreference::Cpu);
        let mut payload = serde_json::to_value(&engine).unwrap();
        let materials = payload["biology"][id.slot() as usize]
            .as_object_mut()
            .unwrap();
        materials.remove("radius");
        let restored = Engine::restore(serde_json::from_value(payload).unwrap()).unwrap();
        assert_eq!(
            restored.slots.get(id).unwrap().materials.radius,
            crate::physics::organism_radius(1000.0, 0.0, false)
        );
    }
}
