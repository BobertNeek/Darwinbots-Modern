//! Quads.bas bucket order, with a transient fine grid to skip distant pairs.
use super::*;
use std::collections::HashSet;

const BUCKET_SIZE: f32 = 4_000.0;
const CONTACT_CELL: f32 = 1_000.0;

#[derive(Clone, Serialize, Deserialize)]
struct Member {
    id: OrganismId,
    cell: [i32; 2],
    order: u64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct NativeBuckets {
    extent: [f32; 2],
    sequence: u64,
    members: Vec<Option<Member>>,
}

impl NativeBuckets {
    fn cell(&self, position: [f32; 2]) -> [i32; 2] {
        std::array::from_fn(|axis| {
            (position[axis] / BUCKET_SIZE).floor().clamp(
                0.0,
                (self.extent[axis] / BUCKET_SIZE).floor().max(1.0) - 1.0,
            ) as i32
        })
    }

    pub(super) fn relocate(&mut self, id: OrganismId, position: [f32; 2]) {
        let cell = self.cell(position);
        let slot = id.slot() as usize;
        self.members
            .resize_with(self.members.len().max(slot + 1), || None);
        if self.members[slot]
            .as_ref()
            .is_some_and(|entry| entry.id == id && entry.cell == cell)
        {
            return;
        }
        self.sequence += 1;
        self.members[slot] = Some(Member {
            id,
            cell,
            order: self.sequence,
        });
    }

    pub(super) fn synchronize(&mut self, extent: [f32; 2], slots: &RobotArena) {
        if self.extent != extent {
            self.extent = extent;
            self.members.clear();
            self.sequence = 0;
        }
        // Fixed robots can retain stale membership after Repel3. Only explicit
        // movement/respawn relocates an existing member, as in UpdateBotBucket.
        for member in &mut self.members {
            if member
                .as_ref()
                .is_some_and(|entry| slots.get(entry.id).is_err())
            {
                *member = None;
            }
        }
        for id in slots.ids_in_slot_order() {
            if self
                .members
                .get(id.slot() as usize)
                .and_then(Option::as_ref)
                .is_none_or(|entry| entry.id != id)
            {
                self.relocate(id, slots.get(id).unwrap().kinematics.position);
            }
        }
    }

    fn neighborhood(&self, id: OrganismId) -> Vec<[i32; 2]> {
        let Some(member) = self
            .members
            .get(id.slot() as usize)
            .and_then(Option::as_ref)
        else {
            return Vec::new();
        };
        let [x, y] = member.cell;
        let size = self
            .extent
            .map(|extent| (extent / BUCKET_SIZE).floor().max(1.0) as i32);
        [
            [0, 0],
            [-1, 0],
            [1, 0],
            [0, -1],
            [0, 1],
            [-1, -1],
            [-1, 1],
            [1, -1],
            [1, 1],
        ]
        .into_iter()
        .map(|[dx, dy]| [x + dx, y + dy])
        .filter(|cell| (0..size[0]).contains(&cell[0]) && (0..size[1]).contains(&cell[1]))
        .collect()
    }

    pub(super) fn order_vision_candidates(&self, actor: OrganismId, candidates: &mut Vec<usize>) {
        let neighborhood = self.neighborhood(actor);
        candidates.retain(|&slot| {
            self.members
                .get(slot)
                .and_then(Option::as_ref)
                .is_some_and(|member| neighborhood.contains(&member.cell))
        });
        // BucketsProximity visits the current bucket, then adjacent buckets,
        // preserving each packed array's insertion order.
        candidates.sort_by_cached_key(|&slot| {
            let member = self.members[slot].as_ref().unwrap();
            (
                neighborhood
                    .iter()
                    .position(|cell| *cell == member.cell)
                    .unwrap(),
                member.order,
            )
        });
    }
}

pub(super) struct NativeContacts {
    cells: HashMap<[i32; 2], HashSet<OrganismId>>,
    locations: HashMap<OrganismId, [i32; 2]>,
    max_radius: f32,
}

impl NativeContacts {
    pub(super) fn new(slots: &RobotArena) -> Self {
        let mut result = Self {
            cells: HashMap::new(),
            locations: HashMap::new(),
            max_radius: 0.0,
        };
        for id in slots.ids_in_slot_order() {
            let robot = slots.get(id).unwrap();
            result.max_radius = result.max_radius.max(robot.materials.radius);
            result.relocate(id, robot.kinematics.position);
        }
        result
    }

    pub(super) fn relocate(&mut self, id: OrganismId, position: [f32; 2]) {
        let cell = position.map(|value| (value / CONTACT_CELL).floor() as i32);
        if let Some(previous) = self.locations.insert(id, cell) {
            if previous == cell {
                return;
            }
            self.cells.get_mut(&previous).unwrap().remove(&id);
        }
        self.cells.entry(cell).or_default().insert(id);
    }

    fn extend_candidates(&self, world: &Engine, actor: OrganismId, walk: &mut ContactWalk) {
        let Ok(robot) = world.slots.get(actor) else {
            return;
        };
        let position = robot.kinematics.position;
        let radius = robot.materials.radius + self.max_radius;
        let low = position.map(|value| ((value - radius) / CONTACT_CELL).floor() as i32);
        let high = position.map(|value| ((value + radius) / CONTACT_CELL).floor() as i32);
        if walk.bounds == Some((low, high)) {
            return;
        }
        walk.bounds = Some((low, high));
        let width = (i64::from(high[0]) - i64::from(low[0]) + 1).max(0) as u64;
        let height = (i64::from(high[1]) - i64::from(low[1]) + 1).max(0) as u64;
        // A malformed/imported giant radius must not enumerate billions of empty cells.
        let query: Vec<_> = if width.saturating_mul(height) > self.cells.len() as u64 * 4 {
            self.cells
                .keys()
                .copied()
                .filter(|cell| {
                    cell[0] >= low[0]
                        && cell[0] <= high[0]
                        && cell[1] >= low[1]
                        && cell[1] <= high[1]
                })
                .collect()
        } else {
            (low[1]..=high[1])
                .flat_map(|y| (low[0]..=high[0]).map(move |x| [x, y]))
                .collect()
        };
        for key in query {
            if !walk.cells.insert(key) {
                continue;
            }
            let Some(cell) = self.cells.get(&key) else {
                continue;
            };
            for &other in cell {
                if other.slot() <= actor.slot() {
                    continue;
                }
                let Some(member) = world
                    .native_buckets
                    .members
                    .get(other.slot() as usize)
                    .and_then(Option::as_ref)
                else {
                    continue;
                };
                let Some(bucket) = walk
                    .neighborhood
                    .iter()
                    .position(|cell| *cell == member.cell)
                else {
                    continue;
                };
                let key = (bucket, member.order);
                if walk.after.is_none_or(|previous| key > previous) {
                    walk.pending.insert(key, other);
                }
            }
        }
    }

    #[cfg(test)]
    fn next_contact(
        &self,
        world: &Engine,
        actor: OrganismId,
        neighborhood: &[[i32; 2]],
        after: Option<(usize, u64)>,
    ) -> Option<((usize, u64), OrganismId)> {
        let mut walk = ContactWalk::new(neighborhood.to_vec());
        walk.after = after;
        self.extend_candidates(world, actor, &mut walk);
        walk.pending
            .into_iter()
            .find(|(_, other)| overlapping(world, actor, *other))
    }
}

struct ContactWalk {
    neighborhood: Vec<[i32; 2]>,
    after: Option<(usize, u64)>,
    cells: HashSet<[i32; 2]>,
    pending: BTreeMap<(usize, u64), OrganismId>,
    bounds: Option<([i32; 2], [i32; 2])>,
}

impl ContactWalk {
    fn new(neighborhood: Vec<[i32; 2]>) -> Self {
        Self {
            neighborhood,
            after: None,
            cells: HashSet::new(),
            pending: BTreeMap::new(),
            bounds: None,
        }
    }
}

fn overlapping(world: &Engine, first: OrganismId, second: OrganismId) -> bool {
    let (Ok(first), Ok(second)) = (world.slots.get(first), world.slots.get(second)) else {
        return false;
    };
    if second.flags.hidden {
        return false;
    }
    let delta = std::array::from_fn::<_, 2, _>(|axis| {
        first.kinematics.position[axis] - second.kinematics.position[axis]
    });
    let distance = first.materials.radius + second.materials.radius;
    delta[0] * delta[0] + delta[1] * delta[1] < distance * distance
}

impl Engine {
    pub(super) fn native_collisions_for(
        &mut self,
        actor: OrganismId,
        contacts: &mut NativeContacts,
    ) {
        let mut walk = ContactWalk::new(self.native_buckets.neighborhood(actor));
        loop {
            contacts.extend_candidates(self, actor, &mut walk);
            let Some((key, other)) = walk.pending.pop_first() else {
                break;
            };
            walk.after = Some(key);
            if !overlapping(self, actor, other) {
                continue;
            }
            let first = self.slots.get(actor).unwrap();
            let second = self.slots.get(other).unwrap();
            let before = [first.kinematics.position, second.kinematics.position];
            let mut positions = before;
            let mut velocities = [first.kinematics.velocity, second.kinematics.velocity];
            let radii = [first.materials.radius, second.materials.radius];
            let masses = [first.materials.mass, second.materials.mass];
            let fixed = [
                robot_is_fixed(first, self.robot_fixing_allowed),
                robot_is_fixed(second, self.robot_fixing_allowed),
            ];
            crate::physics::resolve_collisions(
                &mut positions,
                &mut velocities,
                &radii,
                &masses,
                &fixed,
                &[(0, 1)],
                self.config.physics.elasticity,
            );
            for (index, id) in [actor, other].into_iter().enumerate() {
                let robot = self.slots.get_mut(id).unwrap();
                robot.kinematics.position = positions[index];
                robot.kinematics.velocity = velocities[index];
                write_robot_touch(robot, positions[index], positions[1 - index]);
                robot.senses.touched_opponent = Some([actor, other][1 - index]);
                contacts.relocate(id, positions[index]);
            }
            // Repel3 publishes both contact references in this order, after
            // separation. The second read can observe the first robot new cells.
            self.publish_native_contact_reference(actor, other);
            self.publish_native_contact_reference(other, actor);
            // Repel3 changes physical positions, never coarse bucket membership.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collision_candidates_follow_bucket_order_before_slot_order() {
        let mut world = Engine::new(EngineConfig {
            world_width: 12_000.0,
            world_height: 12_000.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let actor = world.spawn_at(dna.clone(), [3_990.0, 500.0]).unwrap();
        let right = world.spawn_at(dna.clone(), [4_010.0, 500.0]).unwrap();
        let current = world.spawn_at(dna, [3_980.0, 500.0]).unwrap();
        world
            .native_buckets
            .synchronize([12_000.0; 2], &world.slots);
        let contacts = NativeContacts::new(&world.slots);
        let neighborhood = world.native_buckets.neighborhood(actor);
        let (cursor, next) = contacts
            .next_contact(&world, actor, &neighborhood, None)
            .unwrap();
        assert_eq!(next, current);
        assert_eq!(
            contacts
                .next_contact(&world, actor, &neighborhood, Some(cursor))
                .unwrap()
                .1,
            right
        );
    }

    #[test]
    fn bucket_reentry_appends_and_serialized_membership_survives_stale_physical_positions() {
        let mut buckets = NativeBuckets {
            extent: [12_000.0; 2],
            ..Default::default()
        };
        let first = OrganismId::new(0, 0);
        let second = OrganismId::new(1, 0);
        buckets.relocate(first, [500.0; 2]);
        buckets.relocate(second, [500.0; 2]);
        buckets.relocate(first, [4_500.0, 500.0]);
        buckets.relocate(first, [500.0; 2]);
        assert!(
            buckets.members[0].as_ref().unwrap().order > buckets.members[1].as_ref().unwrap().order
        );
        let saved = serde_json::to_string(&buckets).unwrap();
        let restored: NativeBuckets = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.neighborhood(first), buckets.neighborhood(first));
        assert_eq!(
            restored.members[0].as_ref().unwrap().order,
            buckets.members[0].as_ref().unwrap().order
        );
    }

    #[test]
    fn collisions_update_fine_grid_without_relocating_original_buckets() {
        let mut world = Engine::new(EngineConfig {
            world_width: 12_000.0,
            world_height: 12_000.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let dna = LegacyDna::parse("start stop").unwrap();
        let first = world.spawn_at(dna.clone(), [3_995.0, 500.0]).unwrap();
        let second = world.spawn_at(dna, [3_996.0, 500.0]).unwrap();
        world
            .native_buckets
            .synchronize([12_000.0; 2], &world.slots);
        let mut contacts = NativeContacts::new(&world.slots);
        world.native_collisions_for(first, &mut contacts);
        assert!(world.slots.get(second).unwrap().kinematics.position[0] > 4_000.0);
        assert_eq!(
            world.native_buckets.members[second.slot() as usize]
                .as_ref()
                .unwrap()
                .cell,
            [0, 0]
        );
        assert_eq!(contacts.locations[&second], [4, 0]);
    }

    #[test]
    fn separation_discovers_later_contact_in_a_new_fine_cell() {
        let mut world = Engine::new(EngineConfig {
            world_width: 12_000.0,
            world_height: 12_000.0,
            ..EngineConfig::testing()
        })
        .unwrap();
        let positions = [[990.0, 2_000.0], [950.0, 2_000.0], [2_010.0, 2_000.0]];
        let mut ids = Vec::new();
        for position in positions {
            let id = world
                .spawn_at(LegacyDna::parse("start stop").unwrap(), position)
                .unwrap();
            world.slots.get_mut(id).unwrap().materials.radius = 350.0;
            ids.push(id);
        }
        world
            .native_buckets
            .synchronize([12_000.0; 2], &world.slots);
        let mut contacts = NativeContacts::new(&world.slots);
        let mut expected = positions;
        let mut velocity = [[0.0; 2]; 3];
        // Exhaustive Quads order checks both later slots, after each separation.
        crate::physics::resolve_collisions(
            &mut expected,
            &mut velocity,
            &[350.0; 3],
            &[1.0; 3],
            &[false; 3],
            &[(0, 1), (0, 2)],
            world.config.physics.elasticity,
        );
        world.native_collisions_for(ids[0], &mut contacts);
        assert!(expected[2][0] > positions[2][0]);
        for (index, id) in ids.into_iter().enumerate() {
            assert_eq!(
                world.slots.get(id).unwrap().kinematics.position,
                expected[index]
            );
        }
    }
}
