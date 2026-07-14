struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
    slot: u32,
    alive: u32,
    energy: i32,
    radius: f32,
};

struct Params {
    count: u32,
    grid_width: u32,
    grid_height: u32,
    padding: u32,
    cell_size: f32,
    radius_squared: f32,
    world_size: vec2<f32>,
};

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<storage, read_write> cell_counts: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read_write> cell_offsets: array<u32>;
@group(0) @binding(3) var<storage, read_write> cell_cursors: array<atomic<u32>>;
@group(0) @binding(4) var<storage, read_write> cell_members: array<u32>;
@group(0) @binding(5) var<uniform> params: Params;

fn particle_cell(index: u32) -> u32 {
    let cell = vec2<u32>(clamp(floor(particles[index].position / params.cell_size), vec2<f32>(0.0), vec2<f32>(f32(params.grid_width - 1u), f32(params.grid_height - 1u))));
    return cell.y * params.grid_width + cell.x;
}

@compute @workgroup_size(64)
fn clear_grid(@builtin(global_invocation_id) id: vec3<u32>) {
    let cells = params.grid_width * params.grid_height;
    if (id.x < cells) {
        atomicStore(&cell_counts[id.x], 0u);
        atomicStore(&cell_cursors[id.x], 0u);
        cell_offsets[id.x] = 0u;
    }
    if (id.x == 0u) { cell_offsets[cells] = 0u; }
}

@compute @workgroup_size(64)
fn count_particles(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < params.count && particles[id.x].alive != 0u) {
        atomicAdd(&cell_counts[particle_cell(id.x)], 1u);
    }
}

@compute @workgroup_size(1)
fn prefix_offsets(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x != 0u) { return; }
    let cells = params.grid_width * params.grid_height;
    var total = 0u;
    for (var cell = 0u; cell < cells; cell = cell + 1u) {
        cell_offsets[cell] = total;
        total = total + atomicLoad(&cell_counts[cell]);
    }
    cell_offsets[cells] = total;
}

@compute @workgroup_size(64)
fn scatter_members(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < params.count && particles[id.x].alive != 0u) {
        let cell = particle_cell(id.x);
        let member = cell_offsets[cell] + atomicAdd(&cell_cursors[cell], 1u);
        cell_members[member] = id.x;
    }
}
