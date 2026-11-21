use super::*;

pub(super) fn energy_body_credit(body: f32, amount: f64) -> f32 {
    const BODY_SHARE: f64 = 0.029;
    let body = f64::from(body);
    let credit = amount * BODY_SHARE;
    let total = body + credit;
    let rounded = total as f32;
    if total == f64::from(rounded) {
        return rounded;
    }
    let rounds_down = total > f64::from(rounded);
    let other = if rounds_down {
        rounded.next_up()
    } else {
        rounded.next_down()
    };
    if total != (f64::from(rounded) + f64::from(other)) * 0.5 {
        return rounded;
    }

    // Update_Ties retains a 64-bit significand through multiply and add before
    // storing Single. Recover the bits lost by Double at the Single midpoint.
    let credit_step = extended_ulp(credit);
    let credit_remainder =
        (amount.mul_add(BODY_SHARE, -credit) / credit_step).round_ties_even() * credit_step;
    let stored_credit = total - body;
    let sum_remainder = (body - (total - stored_credit)) + (credit - stored_credit);
    let remainder = sum_remainder + credit_remainder;
    let half_step = extended_ulp(total) * 0.5;
    if (rounds_down && remainder > half_step) || (!rounds_down && remainder < -half_step) {
        other
    } else {
        rounded
    }
}

fn extended_ulp(value: f64) -> f64 {
    const EXPONENT_MASK: u64 = 0x7ff0_0000_0000_0000;
    let exponent = value.to_bits() & EXPONENT_MASK;
    f64::from_bits(exponent - (63_u64 << 52))
}

#[derive(Clone, Copy)]
pub(super) enum SharedResource {
    Energy,
    Waste,
    Shell,
    Slime,
    Chloroplasts,
}

impl SharedResource {
    fn field(self, robot: &mut RobotState) -> &mut f32 {
        match self {
            Self::Energy => &mut robot.materials.energy,
            Self::Waste => &mut robot.materials.waste,
            Self::Shell => &mut robot.materials.shell,
            Self::Slime => &mut robot.materials.slime,
            Self::Chloroplasts => &mut robot.materials.chloroplasts,
        }
    }
}

pub(super) fn native_share_resource(
    slots: &mut [RobotSlot],
    source: usize,
    target: usize,
    percent: i32,
    resource: SharedResource,
) -> f32 {
    if percent <= 0 || source == target || source >= slots.len() || target >= slots.len() {
        return 0.0;
    }
    let (source, target) = if source < target {
        let (left, right) = slots.split_at_mut(target);
        (&mut left[source], &mut right[0])
    } else {
        let (left, right) = slots.split_at_mut(source);
        (&mut right[0], &mut left[target])
    };
    let (Some(source), Some(target)) = (source.organism.as_mut(), target.organism.as_mut()) else {
        return 0.0;
    };
    let old = *resource.field(source);
    let other = *resource.field(target);
    let pool = old + other; // Native totnrg/totstuff is Single.
    if matches!(resource, SharedResource::Energy) {
        if old < 0.0 || other < 0.0 {
            return 0.0;
        }
        let percent = if percent % 100 == 0 {
            100
        } else {
            percent % 100
        };
        let desired = (f64::from(pool) * (f64::from(percent) / 100.0)) as f32;
        let mut change = desired.min(32_000.0) - old;
        if change.abs() > source.materials.body {
            change = change.signum() * source.materials.body;
        }
        if old + change > 32_000.0 {
            change = 32_000.0 - old;
        }
        if old + change < 0.0 {
            change = -old;
        }
        if other - change > 32_000.0 {
            change = -(32_000.0 - other);
        }
        if other - change < 0.0 {
            change = other;
        }
        source.materials.energy = old + change;
        target.materials.energy = other - change;
        source.materials.energy =
            (f64::from(source.materials.energy) - f64::from(change.abs()) * 0.01) as f32;
        return change;
    }
    let percent = percent.clamp(0, 99);
    *resource.field(source) =
        ((f64::from(pool) * (f64::from(percent) / 100.0)) as f32).min(32_000.0);
    *resource.field(target) =
        ((f64::from(pool) * (f64::from(100 - percent) / 100.0)) as f32).min(32_000.0);
    if matches!(resource, SharedResource::Shell) {
        source
            .program
            .memory
            .write(823, resource_readback(source.materials.shell));
        target
            .program
            .memory
            .write(823, resource_readback(target.materials.shell));
    }
    *resource.field(source) - old
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_tie_body_credit_retains_native_precision_for_both_transfer_directions() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let id = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [100.0; 2])
            .unwrap();
        let mut receiver = world.slots.get(id).unwrap().clone();
        receiver.materials.body = 249.218_03;
        let mut donor = receiver.clone();
        donor.materials.energy = 2_000.0;
        donor.materials.poison = 0.0;

        for (mut native, expected_body) in [
            (
                Some(crate::native_random::NativeRandom::default()),
                278.218_05,
            ),
            (None, 278.218_02),
        ] {
            for give_energy in [false, true] {
                let mut receiver = receiver.clone();
                let mut donor = donor.clone();
                let effect = if give_energy {
                    apply_tie_energy(&mut donor, &mut receiver, 1_000, &mut native)
                } else {
                    apply_tie_energy(&mut receiver, &mut donor, -1_000, &mut native)
                };
                assert_eq!(effect, Some("tie_resource_energy"));
                assert_eq!(receiver.materials.body, expected_body);
            }
        }
    }

    #[test]
    fn energy_tie_body_credit_preserves_even_rounding_when_the_extended_sum_is_a_tie() {
        let body = 1_000.0_f32;
        let amount = 125.0 / 4096.0;
        assert_eq!(
            energy_body_credit(body, amount).to_bits(),
            body.to_bits() + 14
        );
        assert_eq!(
            energy_body_credit(body.next_up(), amount).to_bits(),
            body.to_bits() + 16
        );
    }

    #[test]
    fn shared_energy_is_a_fraction_of_combined_pool_with_body_limit_and_fee() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let a = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [100.0; 2])
            .unwrap();
        let b = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
            .unwrap();
        world.slots.get_mut(a).unwrap().materials.energy = 500.0;
        world.slots.get_mut(b).unwrap().materials.energy = 1500.0;
        let change = native_share_resource(
            &mut world.slots,
            a.slot() as usize,
            b.slot() as usize,
            50,
            SharedResource::Energy,
        );
        assert_eq!(change, 500.0);
        assert_eq!(world.slots.get(a).unwrap().materials.energy, 995.0);
        assert_eq!(world.slots.get(b).unwrap().materials.energy, 1000.0);
        world.slots.get_mut(a).unwrap().materials.body = 10.0;
        let change = native_share_resource(
            &mut world.slots,
            a.slot() as usize,
            b.slot() as usize,
            100,
            SharedResource::Energy,
        );
        assert_eq!(change, 10.0);
        assert!((world.slots.get(a).unwrap().materials.energy - 1004.9).abs() < 0.001);
    }

    #[test]
    fn shared_shell_assigns_both_sides_of_pool_and_publishes_both_cells() {
        let mut world = Engine::new(EngineConfig::testing()).unwrap();
        let a = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [100.0; 2])
            .unwrap();
        let b = world
            .spawn_at(LegacyDna::parse("start stop").unwrap(), [500.0; 2])
            .unwrap();
        world.slots.get_mut(a).unwrap().materials.shell = 40.0;
        world.slots.get_mut(b).unwrap().materials.shell = 60.0;
        native_share_resource(
            &mut world.slots,
            a.slot() as usize,
            b.slot() as usize,
            25,
            SharedResource::Shell,
        );
        assert_eq!(world.memory_at(a, 823).unwrap(), 25);
        assert_eq!(world.memory_at(b, 823).unwrap(), 75);
    }
}
