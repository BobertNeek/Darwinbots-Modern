use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeVisionPolicy {
    #[default]
    Opaque,
    SeeThrough,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeShotPolicy {
    #[default]
    Absorb,
    /// Modern extension: shots bypass the shape without absorption or reflection.
    PassThrough,
    /// Original nonabsorbing shapes reverse entry axes at the current shot point.
    Reflect,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Obstacle {
    pub id: u32,
    pub minimum: [f32; 2],
    pub maximum: [f32; 2],
    #[serde(default = "default_shape_color")]
    pub color: u32,
    #[serde(default = "default_shape_opacity")]
    pub opacity: f32,
    #[serde(default)]
    pub drift: [f32; 2],
    #[serde(default)]
    pub vision_policy: ShapeVisionPolicy,
    #[serde(default)]
    pub shot_policy: ShapeShotPolicy,
}

impl Obstacle {
    pub fn new(id: u32, minimum: [f32; 2], maximum: [f32; 2]) -> Self {
        Self {
            id,
            minimum,
            maximum,
            color: default_shape_color(),
            opacity: default_shape_opacity(),
            drift: [0.0; 2],
            vision_policy: ShapeVisionPolicy::default(),
            shot_policy: ShapeShotPolicy::default(),
        }
    }
}

fn default_shape_color() -> u32 {
    0xffb9bdb6
}

fn default_shape_opacity() -> f32 {
    1.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Teleporter {
    pub id: u32,
    pub center: [f32; 2],
    pub radius: f32,
    pub destination: [f32; 2],
    /// Native DB2 local gateways choose a new field destination for each transfer.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub random_destination: bool,
    #[serde(default)]
    pub width: f32,
    #[serde(default)]
    pub height: f32,
    /// The original rectangle keeps its upper-left coordinates as Singles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_top_left: Option<[f32; 2]>,
    /// Contact uses the center cached before edge correction in the previous cycle.
    /// The original SIM format omits it. Creation and SIM loading model a virgin VB6 slot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_contact_center: Option<[f32; 2]>,
    #[serde(default = "default_shape_color")]
    pub color: u32,
    #[serde(default)]
    pub velocity: [f32; 2],
    #[serde(default)]
    pub path: PathBuf,
    #[serde(default)]
    pub inbound_path: PathBuf,
    #[serde(default)]
    pub outbound_path: PathBuf,
    #[serde(default = "default_teleporter_inbound")]
    pub inbound: bool,
    #[serde(default)]
    pub outbound: bool,
    #[serde(default = "default_teleporter_local")]
    pub local: bool,
    #[serde(default)]
    pub internet: bool,
    #[serde(default)]
    pub drift_horizontal: bool,
    #[serde(default)]
    pub drift_vertical: bool,
    #[serde(default)]
    pub fixed_position: bool,
    #[serde(default)]
    pub highlight: bool,
    #[serde(default = "default_teleporter_filters")]
    pub filters: TeleporterFilters,
    #[serde(default)]
    pub respect_shapes: bool,
    #[serde(default)]
    pub sent: u64,
    #[serde(default)]
    pub received: u64,
    #[serde(default)]
    pub poll_interval: u32,
    #[serde(default)]
    pub capacity: usize,
    #[serde(default)]
    pub poll_countdown: u32,
    #[serde(default)]
    pub backflow_limit: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_backflow_guard: Option<PathBuf>,
}

impl Teleporter {
    pub fn new(id: u32, center: [f32; 2], radius: f32, destination: [f32; 2]) -> Self {
        Self {
            id,
            center,
            radius,
            destination,
            random_destination: false,
            width: radius * 2.0,
            height: radius * 2.0,
            native_top_left: None,
            native_contact_center: None,
            color: default_shape_color(),
            velocity: [0.0; 2],
            path: PathBuf::new(),
            inbound_path: PathBuf::new(),
            outbound_path: PathBuf::new(),
            inbound: true,
            outbound: false,
            local: true,
            internet: false,
            drift_horizontal: false,
            drift_vertical: false,
            fixed_position: true,
            highlight: false,
            filters: default_teleporter_filters(),
            respect_shapes: false,
            sent: 0,
            received: 0,
            poll_interval: 1,
            capacity: 1,
            poll_countdown: 0,
            backflow_limit: 0,
            file_backflow_guard: None,
        }
    }

    pub(crate) fn native_contact_center_at(&self, position: [f32; 2]) -> [f32; 2] {
        [
            (f64::from(position[0]) + f64::from(self.width) * 0.5) as f32,
            (f64::from(position[1]) + f64::from(self.height) * 0.3) as f32,
        ]
    }

    pub(crate) fn native_contact(&self) -> Option<NativeTeleporterContact> {
        let anchor = self.native_top_left.filter(|_| !self.internet)?;
        Some(NativeTeleporterContact {
            // Older Modern checkpoints have only the corrected anchor. The exact
            // pre-wrap center is unrecoverable until the next movement phase.
            center: self
                .native_contact_center
                .unwrap_or_else(|| self.native_contact_center_at(anchor)),
            width: self.width,
        })
    }
}

impl Default for Teleporter {
    fn default() -> Self {
        Self::new(0, [0.0; 2], 1.0, [0.0; 2])
    }
}

fn default_teleporter_inbound() -> bool {
    true
}

fn default_teleporter_local() -> bool {
    true
}

fn default_teleporter_filters() -> TeleporterFilters {
    TeleporterFilters {
        teleport_vegetables: true,
        teleport_corpses: true,
        teleport_heterotrophs: true,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TeleporterFilters {
    pub teleport_vegetables: bool,
    pub teleport_corpses: bool,
    pub teleport_heterotrophs: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeTeleporterKind {
    Intrasim,
    Inbound,
    Outbound,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NativeTeleporterOptions {
    pub kind: NativeTeleporterKind,
    pub size: u32,
    pub path: PathBuf,
    pub fixed_position: bool,
    pub filters: TeleporterFilters,
    pub poll_interval: u32,
    pub capacity: u32,
}

impl TeleporterFilters {
    pub(crate) fn accepts(&self, vegetable: bool, corpse: bool) -> bool {
        if corpse {
            self.teleport_corpses
        } else if vegetable {
            self.teleport_vegetables
        } else {
            self.teleport_heterotrophs
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TeleporterBackflow {
    pub(crate) source: [f32; 2],
    pub(crate) remaining: u32,
}

#[derive(Clone, Copy)]
pub(crate) struct NativeTeleporterContact {
    pub(crate) center: [f32; 2],
    width: f32,
}

impl NativeTeleporterContact {
    pub(crate) fn touches(self, position: [f32; 2], robot_radius: f32) -> bool {
        let relative = [position[0] - self.center[0], position[1] - self.center[1]];
        f64::from(crate::physics::native_magnitude(relative))
            < f64::from(self.width) / 2.0 + f64::from(robot_radius)
    }
}

pub(crate) fn inside_teleporter(
    teleporter: &Teleporter,
    position: [f32; 2],
    robot_radius: f32,
) -> bool {
    if teleporter.internet {
        return false;
    }
    if let Some(contact) = teleporter.native_contact() {
        return contact.touches(position, robot_radius);
    }
    let dx = position[0] - teleporter.center[0];
    let dy = position[1] - teleporter.center[1];
    dx * dx + dy * dy <= teleporter.radius * teleporter.radius
}

pub(crate) fn teleporter_blocked_by_obstacles(
    teleporter: &Teleporter,
    obstacles: &[Obstacle],
) -> bool {
    if !teleporter.respect_shapes {
        return false;
    }
    obstacles.iter().any(|obstacle| {
        let nearest_x = teleporter.center[0].clamp(obstacle.minimum[0], obstacle.maximum[0]);
        let nearest_y = teleporter.center[1].clamp(obstacle.minimum[1], obstacle.maximum[1]);
        let dx = teleporter.center[0] - nearest_x;
        let dy = teleporter.center[1] - nearest_y;
        dx * dx + dy * dy <= teleporter.radius * teleporter.radius
    })
}

pub(crate) fn blocks_backflow(
    teleporter: &Teleporter,
    backflow: Option<TeleporterBackflow>,
) -> bool {
    if teleporter.random_destination {
        return false;
    }
    let Some(backflow) = backflow else {
        return false;
    };
    if backflow.remaining == 0 {
        return false;
    }
    let dx = teleporter.destination[0] - backflow.source[0];
    let dy = teleporter.destination[1] - backflow.source[1];
    dx * dx + dy * dy <= teleporter.radius * teleporter.radius
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FileTeleporter {
    pub id: u32,
    pub center: [f32; 2],
    pub radius: f32,
    pub inbound_path: PathBuf,
    pub outbound_path: PathBuf,
    pub poll_interval: u32,
    pub poll_countdown: u32,
    pub capacity: usize,
    pub filters: TeleporterFilters,
    pub sent: u64,
    pub received: u64,
    #[serde(default)]
    pub(crate) backflow_guard: Option<PathBuf>,
}

impl FileTeleporter {
    pub fn new(id: u32, center: [f32; 2], radius: f32) -> Self {
        Self {
            id,
            center,
            radius,
            inbound_path: PathBuf::new(),
            outbound_path: PathBuf::new(),
            poll_interval: 1,
            poll_countdown: 0,
            capacity: 1,
            filters: TeleporterFilters {
                teleport_vegetables: true,
                teleport_corpses: true,
                teleport_heterotrophs: true,
            },
            sent: 0,
            received: 0,
            backflow_guard: None,
        }
    }

    pub fn with_paths(
        mut self,
        inbound_path: impl AsRef<Path>,
        outbound_path: impl AsRef<Path>,
    ) -> Self {
        self.inbound_path = inbound_path.as_ref().to_path_buf();
        self.outbound_path = outbound_path.as_ref().to_path_buf();
        self
    }

    pub fn with_polling(mut self, poll_interval: u32, capacity: usize) -> Self {
        self.poll_interval = poll_interval;
        self.capacity = capacity;
        self
    }

    pub fn with_filters(mut self, filters: TeleporterFilters) -> Self {
        self.filters = filters;
        self
    }

    pub fn backflow_guard_active(&self) -> bool {
        self.backflow_guard.is_some()
    }

    pub(crate) fn accepts(&self, vegetable: bool, corpse: bool) -> bool {
        if corpse {
            self.filters.teleport_corpses
        } else if vegetable {
            self.filters.teleport_vegetables
        } else {
            self.filters.teleport_heterotrophs
        }
    }
}

pub(crate) fn apply_world_features(
    radii: &[f32],
    positions: &mut [[f32; 2]],
    velocities: &mut [[f32; 2]],
    alive: &[bool],
    obstacles: &[Obstacle],
    teleporters: &[Teleporter],
    world_size: [f32; 2],
) {
    for slot in 0..positions.len() {
        if !alive.get(slot).copied().unwrap_or(false) {
            continue;
        }
        for obstacle in obstacles {
            let radius = radii.get(slot).copied().unwrap_or_default().max(0.0);
            let obstacle = Obstacle {
                minimum: [obstacle.minimum[0] - radius, obstacle.minimum[1] - radius],
                maximum: [obstacle.maximum[0] + radius, obstacle.maximum[1] + radius],
                ..obstacle.clone()
            };
            let position = &mut positions[slot];
            if position[0] < obstacle.minimum[0]
                || position[0] > obstacle.maximum[0]
                || position[1] < obstacle.minimum[1]
                || position[1] > obstacle.maximum[1]
            {
                continue;
            }
            let distances = [
                (position[0] - obstacle.minimum[0]).abs(),
                (obstacle.maximum[0] - position[0]).abs(),
                (position[1] - obstacle.minimum[1]).abs(),
                (obstacle.maximum[1] - position[1]).abs(),
            ];
            let edge = distances
                .iter()
                .enumerate()
                .min_by(|left, right| left.1.total_cmp(right.1))
                .map(|value| value.0)
                .unwrap_or(0);
            match edge {
                0 => {
                    position[0] = obstacle.minimum[0];
                    velocities[slot][0] = -velocities[slot][0].abs();
                }
                1 => {
                    position[0] = obstacle.maximum[0];
                    velocities[slot][0] = velocities[slot][0].abs();
                }
                2 => {
                    position[1] = obstacle.minimum[1];
                    velocities[slot][1] = -velocities[slot][1].abs();
                }
                _ => {
                    position[1] = obstacle.maximum[1];
                    velocities[slot][1] = velocities[slot][1].abs();
                }
            }
        }
        for teleporter in teleporters {
            let dx = positions[slot][0] - teleporter.center[0];
            let dy = positions[slot][1] - teleporter.center[1];
            if dx * dx + dy * dy <= teleporter.radius * teleporter.radius {
                positions[slot] = [
                    teleporter.destination[0].clamp(0.0, world_size[0]),
                    teleporter.destination[1].clamp(0.0, world_size[1]),
                ];
                break;
            }
        }
    }
}
