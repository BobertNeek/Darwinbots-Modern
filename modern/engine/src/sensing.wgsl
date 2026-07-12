struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    slot: u32,
    alive: u32,
    energy: i32,
    _padding: u32,
};

struct Params {
    count: u32,
    grid_width: u32,
    grid_height: u32,
    _padding: u32,
    cell_size: f32,
    radius_squared: f32,
    world_size: vec2<f32>,
};

struct SenseOutput {
    nearest_slot: u32,
    _padding: u32,
    position: vec2<f32>,
    radius: f32,
    color: u32,
    slot: u32,
    _padding2: u32,
};

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read> cell_offsets: array<u32>;
@group(0) @binding(2) var<storage, read> cell_members: array<u32>;
@group(0) @binding(3) var<storage, read_write> outputs: array<SenseOutput>;
@group(0) @binding(4) var<uniform> params: Params;

var<workgroup> lane_distances: array<f32, 64>;
var<workgroup> lane_slots: array<u32, 64>;

@compute @workgroup_size(64)
fn main(
    @builtin(workgroup_id) group: vec3<u32>,
    @builtin(local_invocation_id) local: vec3<u32>,
) {
    let organism_in_group = local.x / 4u;
    let lane = local.x % 4u;
    let index = group.x * 16u + organism_in_group;
    var best_slot = 0xffffffffu;
    var best_distance = params.radius_squared;
    if (index < params.count) {
        let observer = particles[index];
        if (observer.alive != 0u) {
            let center = vec2<i32>(floor(observer.position / params.cell_size));
            let rings = i32(ceil(sqrt(params.radius_squared) / params.cell_size));
            for (var dy = -rings; dy <= rings; dy = dy + 1) {
                for (var dx = -rings; dx <= rings; dx = dx + 1) {
                    let cell = center + vec2<i32>(dx, dy);
                    if (cell.x < 0 || cell.y < 0 || cell.x >= i32(params.grid_width) || cell.y >= i32(params.grid_height)) { continue; }
                    let cell_index = u32(cell.y) * params.grid_width + u32(cell.x);
                    let start = cell_offsets[cell_index];
                    let end = cell_offsets[cell_index + 1u];
                    for (var member_index = start + lane; member_index < end; member_index = member_index + 4u) {
                        let candidate_index = cell_members[member_index];
                        let candidate = particles[candidate_index];
                        if (candidate.alive == 0u || candidate.slot == observer.slot) { continue; }
                        let delta = candidate.position - observer.position;
                        let distance = dot(delta, delta);
                        if (distance < best_distance || (distance == best_distance && candidate.slot < best_slot)) {
                            best_distance = distance;
                            best_slot = candidate.slot;
                        }
                    }
                }
            }
        }
    }
    lane_distances[local.x] = best_distance;
    lane_slots[local.x] = best_slot;
    workgroupBarrier();
    if (lane == 0u && index < params.count) {
        for (var other = 1u; other < 4u; other = other + 1u) {
            let other_distance = lane_distances[local.x + other];
            let other_slot = lane_slots[local.x + other];
            if (other_distance < best_distance || (other_distance == best_distance && other_slot < best_slot)) {
                best_distance = other_distance;
                best_slot = other_slot;
            }
        }
        let observer = particles[index];
        outputs[index].nearest_slot = best_slot;
        outputs[index].position = select(
            observer.position,
            clamp(observer.position + observer.velocity, vec2<f32>(0.0), params.world_size),
            observer.alive != 0u,
        );
        outputs[index].radius = clamp(sqrt(f32(max(observer.energy, 1))) * 0.45, 2.0, 24.0);
        let energy_color = u32(clamp(observer.energy, 0, 4000) * 255 / 4000);
        outputs[index].color = 0xff2f8020u + (energy_color << 8u);
        outputs[index].slot = observer.slot;
    }
}
