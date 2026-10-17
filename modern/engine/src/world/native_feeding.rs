//! Shots.releasebod resource assignments, including the native leftover rules.

use crate::BiologyState;

pub(super) fn release_body(materials: &mut BiologyState, corpse: bool, power: f32) -> f32 {
    if materials.body <= 0.0 || power <= 0.0 {
        return 0.0;
    }
    let shell = materials.shell * 20.0;
    let mut power = power.min(32_000.0);
    let maximum = (f64::from(materials.body * 10.0) / 0.8 + f64::from(shell)) as f32;
    power = power.min(maximum);
    materials.shell = (materials.shell - power / 20.0).max(0.0);
    if power < shell {
        return 0.0;
    }
    power -= shell;
    if power <= 0.0 {
        return 0.0;
    }
    if corpse {
        power = (power * 4.0).min(materials.body * 10.0);
        materials.body -= power / 10.0;
    } else {
        let mut leftover = 0.0;
        let energy_lost = (f64::from(power) * 0.2) as f32;
        if energy_lost > materials.energy {
            leftover = energy_lost - materials.energy;
            materials.energy = 0.0;
        } else {
            materials.energy -= energy_lost;
        }
        let body_lost = (f64::from(power) * 0.08) as f32;
        if body_lost > materials.body {
            // The comparison and the multiplied subtraction differ in VB6.
            // Preserve that behavior rather than silently correcting it.
            leftover += body_lost - materials.body * 10.0;
            materials.body = 0.0;
        } else {
            materials.body -= body_lost;
        }
        if leftover > 0.0 {
            if materials.energy > 0.0 && materials.energy > leftover {
                materials.energy -= leftover;
                leftover = 0.0;
            } else if materials.energy > 0.0 && materials.energy < leftover {
                leftover -= materials.energy;
                materials.energy = 0.0;
            }
            if materials.body > 0.0 && materials.body * 10.0 > leftover {
                materials.body = (f64::from(materials.body) - f64::from(leftover) * 0.1) as f32;
            } else if materials.body > 0.0 && materials.body * 10.0 < leftover {
                materials.body = 0.0;
            }
        }
    }
    power
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_feed_caps_the_return_packet_and_obeys_explicit_shot_death() {
        use super::super::*;
        let mut world = Engine::new(EngineConfig {
            corpse_decay: CorpseDecaySettings {
                enabled: true,
                ..Default::default()
            },
            ..EngineConfig::testing()
        })
        .unwrap();
        let owner = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [300.0, 500.0])
            .unwrap();
        let target = world
            .spawn_at(LegacyDna::parse("stop").unwrap(), [600.0, 500.0])
            .unwrap();
        let robot = world.slots.get_mut(target).unwrap();
        robot.materials.energy = 10.0;
        robot.materials.body = 100.0;
        // Preserve the return packet while leaving room for KillRobot's 20 poffs.
        world.projectiles.reset_native_slots(100);
        assert!(world.apply_projectile_effect(
            owner,
            ProjectileTarget::Organism(target.slot() as usize),
            ProjectileEffect::ReleaseEnergy,
            100.0,
            0.0,
            None,
            None,
            0,
            0,
            [600.0, 500.0],
            [40.0, 0.0],
            10.0,
            400.0
        ));
        let robot = world.slots.get(target).unwrap();
        assert_eq!(robot.materials.energy, 0.0);
        assert_eq!(robot.materials.body, 99.9);
        assert!(
            robot.flags.dead,
            "releasenrg explicitly marks a lethal hit Dead before ManageDeath"
        );
        assert_eq!(world.projectiles.energies[0], 10.0);
        assert_eq!(world.projectiles.values[0], 10);
        assert_eq!(world.slots.get(owner).unwrap().counters.kills, 1);
        world.tick().unwrap();
        assert!(world.slots.get(target).is_err());
        assert!(world.snapshot().corpses.is_empty());
    }

    #[test]
    fn body_shot_takes_eight_percent_as_body_points_and_corpse_uses_fourfold_power() {
        for (corpse, expected) in [(false, [950.0, 80.0, 250.0]), (true, [1000.0, 0.0, 1000.0])] {
            let mut stores = BiologyState {
                energy: 1000.0,
                body: 100.0,
                ..Default::default()
            };
            let packet = release_body(&mut stores, corpse, 250.0);
            assert_eq!([stores.energy, stores.body, packet], expected);
        }
    }

    #[test]
    fn body_shot_respects_shell_equality_and_native_leftover_equality() {
        let mut stores = BiologyState {
            energy: 1000.0,
            body: 100.0,
            shell: 5.0,
            ..Default::default()
        };
        assert_eq!(release_body(&mut stores, false, 100.0), 0.0);
        assert_eq!(
            [stores.energy, stores.body, stores.shell],
            [1000.0, 100.0, 0.0]
        );
        let mut stores = BiologyState {
            energy: 25.0,
            body: 7.5,
            ..Default::default()
        };
        // Native cap=93.75, loss=18.75 energy and 7.5 body: equality empties body.
        assert_eq!(release_body(&mut stores, false, 250.0), 93.75);
        assert_eq!([stores.energy, stores.body], [6.25, 0.0]);
    }
}
