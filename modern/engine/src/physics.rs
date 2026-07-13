use crate::EngineError;
use bytemuck::{Pod, Zeroable};
use std::sync::mpsc;
use wgpu::util::DeviceExt;

mod movement;
mod environment;
mod collision;

pub(crate) use movement::{
    MovementInput, MovementState, apply_voluntary_impulse, derived_mass,
};
pub(crate) use environment::{apply_resistance, environment_impulse, integrate_body};
pub(crate) use collision::resolve_collisions;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuSenseParticle {
    position: [f32; 2],
    velocity: [f32; 2],
    slot: u32,
    alive: u32,
    energy: i32,
    _padding: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuSenseParams {
    count: u32,
    grid_width: u32,
    grid_height: u32,
    _padding: u32,
    cell_size: f32,
    radius_squared: f32,
    world_size: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuSenseOutput {
    nearest_slot: u32,
    _padding: u32,
    position: [f32; 2],
    radius: f32,
    color: u32,
    slot: u32,
    _padding2: u32,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RenderInstance {
    pub slot: u32,
    pub position: [f32; 2],
    pub radius: f32,
    pub color: u32,
}

struct GpuSensingBuffers {
    particle_capacity: usize,
    offset_capacity: usize,
    member_capacity: usize,
    particles: wgpu::Buffer,
    offsets: wgpu::Buffer,
    members: wgpu::Buffer,
    _counts: wgpu::Buffer,
    _cursors: wgpu::Buffer,
    output: wgpu::Buffer,
    params: wgpu::Buffer,
    readback: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    grid_bind_group: wgpu::BindGroup,
}

impl GpuSensingBuffers {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        grid_layout: &wgpu::BindGroupLayout,
        particle_capacity: usize,
        offset_capacity: usize,
        member_capacity: usize,
    ) -> Self {
        let particle_capacity = particle_capacity.max(1).next_power_of_two();
        let offset_capacity = offset_capacity.max(1).next_power_of_two();
        let member_capacity = member_capacity.max(1).next_power_of_two();
        let particles = storage_upload_buffer(device, "darwinbots sensing particles", particle_capacity * std::mem::size_of::<GpuSenseParticle>());
        let offsets = storage_upload_buffer(device, "darwinbots sensing offsets", offset_capacity * std::mem::size_of::<u32>());
        let members = storage_upload_buffer(device, "darwinbots sensing members", member_capacity * std::mem::size_of::<u32>());
        let counts = storage_upload_buffer(device, "darwinbots grid counts", offset_capacity * std::mem::size_of::<u32>());
        let cursors = storage_upload_buffer(device, "darwinbots grid cursors", offset_capacity * std::mem::size_of::<u32>());
        let output_size = (particle_capacity * std::mem::size_of::<GpuSenseOutput>()) as u64;
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("darwinbots sensing output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("darwinbots sensing parameters"),
            size: std::mem::size_of::<GpuSenseParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("darwinbots sensing readback"),
            size: output_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("darwinbots sensing bind group"),
            layout,
            entries: &[
                buffer_entry(0, &particles),
                buffer_entry(1, &offsets),
                buffer_entry(2, &members),
                buffer_entry(3, &output),
                buffer_entry(4, &params),
            ],
        });
        let grid_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("darwinbots grid bind group"),
            layout: grid_layout,
            entries: &[
                buffer_entry(0, &particles),
                buffer_entry(1, &counts),
                buffer_entry(2, &offsets),
                buffer_entry(3, &cursors),
                buffer_entry(4, &members),
                buffer_entry(5, &params),
            ],
        });
        Self {
            particle_capacity, offset_capacity, member_capacity, particles, offsets, members, _counts: counts, _cursors: cursors,
            output, params, readback, bind_group, grid_bind_group,
        }
    }
}

fn storage_upload_buffer(device: &wgpu::Device, label: &'static str, size: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: size as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn storage_layout_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn buffer_entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource: buffer.as_entire_binding() }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicsBatch {
    pub positions: Vec<[f32; 2]>,
    pub velocities: Vec<[f32; 2]>,
    pub world_size: [f32; 2],
}

pub trait PhysicsBackend {
    fn step(&mut self, batch: &mut PhysicsBatch) -> Result<(), EngineError>;
}

pub(crate) fn organism_radius(energy: i32) -> f32 {
    (energy.max(1) as f32).sqrt().mul_add(0.45, 0.0).clamp(2.0, 24.0)
}

#[derive(Default)]
pub struct CpuPhysicsBackend;

impl PhysicsBackend for CpuPhysicsBackend {
    fn step(&mut self, batch: &mut PhysicsBatch) -> Result<(), EngineError> {
        validate_batch(batch)?;
        for (position, velocity) in batch.positions.iter_mut().zip(&batch.velocities) {
            integrate_body(position, *velocity, batch.world_size);
        }
        Ok(())
    }
}

pub struct GpuPhysicsBackend {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sensing_pipeline: wgpu::ComputePipeline,
    sensing_bind_group_layout: wgpu::BindGroupLayout,
    grid_bind_group_layout: wgpu::BindGroupLayout,
    grid_clear_pipeline: wgpu::ComputePipeline,
    grid_count_pipeline: wgpu::ComputePipeline,
    grid_prefix_pipeline: wgpu::ComputePipeline,
    grid_scatter_pipeline: wgpu::ComputePipeline,
    sensing_buffers: std::sync::Mutex<Option<GpuSensingBuffers>>,
}

impl GpuPhysicsBackend {
    pub fn new() -> Result<Self, EngineError> {
        pollster::block_on(Self::new_async())
    }

    async fn new_async() -> Result<Self, EngineError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|error| EngineError::GpuUnavailable(error.to_string()))?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("darwinbots compute device"),
                ..Default::default()
            })
            .await
            .map_err(|error| EngineError::GpuUnavailable(error.to_string()))?;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("darwinbots physics shader"),
            source: wgpu::ShaderSource::Wgsl(PHYSICS_SHADER.into()),
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("darwinbots physics bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("darwinbots physics pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("darwinbots physics pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let sensing_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("darwinbots sensing shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sensing.wgsl").into()),
        });
        let sensing_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("darwinbots sensing bind group layout"),
            entries: &[
                storage_layout_entry(0, true),
                storage_layout_entry(1, true),
                storage_layout_entry(2, true),
                storage_layout_entry(3, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let sensing_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("darwinbots sensing pipeline layout"),
            bind_group_layouts: &[Some(&sensing_bind_group_layout)],
            immediate_size: 0,
        });
        let sensing_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("darwinbots sensing pipeline"),
            layout: Some(&sensing_pipeline_layout),
            module: &sensing_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let grid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("darwinbots spatial grid shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("spatial_grid.wgsl").into()),
        });
        let grid_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("darwinbots spatial grid bind group layout"),
            entries: &[
                storage_layout_entry(0, true),
                storage_layout_entry(1, false),
                storage_layout_entry(2, false),
                storage_layout_entry(3, false),
                storage_layout_entry(4, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let grid_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("darwinbots spatial grid pipeline layout"),
            bind_group_layouts: &[Some(&grid_bind_group_layout)],
            immediate_size: 0,
        });
        let make_grid_pipeline = |label: &'static str, entry: &'static str| device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(label), layout: Some(&grid_pipeline_layout), module: &grid_shader,
            entry_point: Some(entry), compilation_options: Default::default(), cache: None,
        });
        let grid_clear_pipeline = make_grid_pipeline("darwinbots grid clear pipeline", "clear_grid");
        let grid_count_pipeline = make_grid_pipeline("darwinbots grid count pipeline", "count_particles");
        let grid_prefix_pipeline = make_grid_pipeline("darwinbots grid prefix pipeline", "prefix_offsets");
        let grid_scatter_pipeline = make_grid_pipeline("darwinbots grid scatter pipeline", "scatter_members");
        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group_layout,
            sensing_pipeline,
            sensing_bind_group_layout,
            grid_bind_group_layout,
            grid_clear_pipeline,
            grid_count_pipeline,
            grid_prefix_pipeline,
            grid_scatter_pipeline,
            sensing_buffers: std::sync::Mutex::new(None),
        })
    }

    pub fn sense_nearest(
        &self,
        positions: &[[f32; 2]],
        alive: &[bool],
        cell_offsets: &[u32],
        cell_members: &[u32],
        grid_size: [u32; 2],
        cell_size: f32,
        radius: f32,
    ) -> Result<Vec<Option<usize>>, EngineError> {
        let velocities = vec![[0.0; 2]; positions.len()];
        self.sense_and_integrate(
            positions, &velocities, alive, cell_offsets, cell_members, grid_size,
            cell_size, radius, [f32::MAX, f32::MAX],
        ).map(|result| result.0)
    }

    pub fn sense_nearest_gpu_grid(
        &self,
        positions: &[[f32; 2]],
        alive: &[bool],
        world_size: [f32; 2],
        cell_size: f32,
        radius: f32,
    ) -> Result<Vec<Option<usize>>, EngineError> {
        let velocities = vec![[0.0; 2]; positions.len()];
        let energies = vec![0; positions.len()];
        self.sense_integrate_render_gpu_grid(
            positions, &velocities, &energies, alive, world_size, cell_size, radius,
        ).map(|result| result.0)
    }

    pub fn sense_integrate_render_gpu_grid(
        &self,
        positions: &[[f32; 2]],
        velocities: &[[f32; 2]],
        energies: &[i32],
        alive: &[bool],
        world_size: [f32; 2],
        cell_size: f32,
        radius: f32,
    ) -> Result<(Vec<Option<usize>>, Vec<[f32; 2]>, Vec<RenderInstance>), EngineError> {
        let grid_size = [
            (world_size[0] / cell_size).ceil().max(1.0) as u32,
            (world_size[1] / cell_size).ceil().max(1.0) as u32,
        ];
        self.sense_integrate_render_impl(
            positions, velocities, energies, alive, &[], &[], grid_size,
            cell_size, radius, world_size, true,
        )
    }

    pub fn sense_and_integrate(
        &self,
        positions: &[[f32; 2]],
        velocities: &[[f32; 2]],
        alive: &[bool],
        cell_offsets: &[u32],
        cell_members: &[u32],
        grid_size: [u32; 2],
        cell_size: f32,
        radius: f32,
        world_size: [f32; 2],
    ) -> Result<(Vec<Option<usize>>, Vec<[f32; 2]>), EngineError> {
        let energies = vec![0; positions.len()];
        self.sense_integrate_render(
            positions, velocities, &energies, alive, cell_offsets, cell_members,
            grid_size, cell_size, radius, world_size,
        ).map(|(targets, positions, _)| (targets, positions))
    }

    pub fn sense_integrate_render(
        &self,
        positions: &[[f32; 2]],
        velocities: &[[f32; 2]],
        energies: &[i32],
        alive: &[bool],
        cell_offsets: &[u32],
        cell_members: &[u32],
        grid_size: [u32; 2],
        cell_size: f32,
        radius: f32,
        world_size: [f32; 2],
    ) -> Result<(Vec<Option<usize>>, Vec<[f32; 2]>, Vec<RenderInstance>), EngineError> {
        self.sense_integrate_render_impl(
            positions, velocities, energies, alive, cell_offsets, cell_members, grid_size,
            cell_size, radius, world_size, false,
        )
    }

    fn sense_integrate_render_impl(
        &self,
        positions: &[[f32; 2]],
        velocities: &[[f32; 2]],
        energies: &[i32],
        alive: &[bool],
        cell_offsets: &[u32],
        cell_members: &[u32],
        grid_size: [u32; 2],
        cell_size: f32,
        radius: f32,
        world_size: [f32; 2],
        gpu_build_grid: bool,
    ) -> Result<(Vec<Option<usize>>, Vec<[f32; 2]>, Vec<RenderInstance>), EngineError> {
        if positions.is_empty() {
            return Ok((Vec::new(), Vec::new(), Vec::new()));
        }
        let particles: Vec<_> = positions.iter().enumerate().map(|(slot, position)| GpuSenseParticle {
            position: *position,
            velocity: velocities.get(slot).copied().unwrap_or([0.0; 2]),
            slot: slot as u32,
            alive: alive.get(slot).copied().unwrap_or(false) as u32,
            energy: energies.get(slot).copied().unwrap_or(0),
            _padding: 0,
        }).collect();
        let params = GpuSenseParams {
            count: particles.len() as u32,
            grid_width: grid_size[0],
            grid_height: grid_size[1],
            _padding: 0,
            cell_size,
            radius_squared: radius * radius,
            world_size,
        };
        let sentinel_members = [0u32];
        let members = if cell_members.is_empty() { &sentinel_members[..] } else { cell_members };
        let grid_cells = (grid_size[0] as usize).saturating_mul(grid_size[1] as usize);
        let required_offsets = if gpu_build_grid { grid_cells + 1 } else { cell_offsets.len() };
        let required_members = if gpu_build_grid { positions.len().max(1) } else { members.len() };
        let output_size = (particles.len() * std::mem::size_of::<GpuSenseOutput>()) as u64;
        let mut guard = self.sensing_buffers.lock().map_err(|error| EngineError::Gpu(error.to_string()))?;
        let needs_growth = guard.as_ref().is_none_or(|buffers| {
            buffers.particle_capacity < particles.len()
                || buffers.offset_capacity < required_offsets
                || buffers.member_capacity < required_members
        });
        if needs_growth {
            *guard = Some(GpuSensingBuffers::new(
                &self.device,
                &self.sensing_bind_group_layout,
                &self.grid_bind_group_layout,
                particles.len(),
                required_offsets,
                required_members,
            ));
        }
        let buffers = guard.as_ref().unwrap();
        self.queue.write_buffer(&buffers.particles, 0, bytemuck::cast_slice(&particles));
        if !gpu_build_grid {
            self.queue.write_buffer(&buffers.offsets, 0, bytemuck::cast_slice(cell_offsets));
            self.queue.write_buffer(&buffers.members, 0, bytemuck::cast_slice(members));
        }
        self.queue.write_buffer(&buffers.params, 0, bytemuck::bytes_of(&params));
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("darwinbots sensing encoder"),
        });
        if gpu_build_grid {
            for (label, pipeline, workgroups) in [
                ("darwinbots grid clear pass", &self.grid_clear_pipeline, (grid_cells as u32).div_ceil(64)),
                ("darwinbots grid count pass", &self.grid_count_pipeline, params.count.div_ceil(64)),
                ("darwinbots grid prefix pass", &self.grid_prefix_pipeline, 1),
                ("darwinbots grid scatter pass", &self.grid_scatter_pipeline, params.count.div_ceil(64)),
            ] {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some(label), timestamp_writes: None });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &buffers.grid_bind_group, &[]);
                pass.dispatch_workgroups(workgroups.max(1), 1, 1);
            }
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("darwinbots sensing pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.sensing_pipeline);
            pass.set_bind_group(0, &buffers.bind_group, &[]);
            pass.dispatch_workgroups(params.count.div_ceil(16), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&buffers.output, 0, &buffers.readback, 0, output_size);
        self.queue.submit(Some(encoder.finish()));
        let slice = buffers.readback.slice(..output_size);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| { let _ = sender.send(result); });
        self.device.poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| EngineError::Gpu(error.to_string()))?;
        receiver.recv().map_err(|error| EngineError::Gpu(error.to_string()))?
            .map_err(|error| EngineError::Gpu(error.to_string()))?;
        let mapped = slice.get_mapped_range();
        let outputs = bytemuck::cast_slice::<u8, GpuSenseOutput>(&mapped);
        let result = outputs.iter()
            .map(|output| (output.nearest_slot != u32::MAX).then_some(output.nearest_slot as usize))
            .collect();
        let integrated = outputs.iter().map(|output| output.position).collect();
        let render_instances = outputs.iter().enumerate().filter_map(|(index, output)| {
            alive.get(index).copied().unwrap_or(false).then_some(RenderInstance {
                slot: output.slot,
                position: output.position,
                radius: output.radius,
                color: output.color,
            })
        }).collect();
        drop(mapped);
        buffers.readback.unmap();
        Ok((result, integrated, render_instances))
    }
}

impl PhysicsBackend for GpuPhysicsBackend {
    fn step(&mut self, batch: &mut PhysicsBatch) -> Result<(), EngineError> {
        validate_batch(batch)?;
        if batch.positions.is_empty() {
            return Ok(());
        }

        let particles: Vec<_> = batch.positions.iter().zip(&batch.velocities)
            .map(|(position, velocity)| GpuParticle { position: *position, velocity: *velocity })
            .collect();
        let params = GpuParams {
            world_size: batch.world_size,
            count: particles.len() as u32,
            _padding: 0,
        };
        let particle_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("darwinbots particles"),
            contents: bytemuck::cast_slice(&particles),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        let params_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("darwinbots physics parameters"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let output_size = std::mem::size_of_val(particles.as_slice()) as u64;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("darwinbots physics readback"),
            size: output_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("darwinbots physics bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: particle_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: params_buffer.as_entire_binding() },
            ],
        });
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("darwinbots physics encoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("darwinbots physics pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((particles.len() as u32).div_ceil(64), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&particle_buffer, 0, &readback, 0, output_size);
        self.queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::PollType::wait_indefinitely())
            .map_err(|error| EngineError::Gpu(error.to_string()))?;
        receiver.recv().map_err(|error| EngineError::Gpu(error.to_string()))?
            .map_err(|error| EngineError::Gpu(error.to_string()))?;
        let mapped = slice.get_mapped_range();
        let result: &[GpuParticle] = bytemuck::cast_slice(&mapped);
        for (destination, particle) in batch.positions.iter_mut().zip(result) {
            *destination = particle.position;
        }
        drop(mapped);
        readback.unmap();
        Ok(())
    }
}

fn validate_batch(batch: &PhysicsBatch) -> Result<(), EngineError> {
    if batch.positions.len() != batch.velocities.len() {
        return Err(EngineError::Gpu("position and velocity counts differ".to_owned()));
    }
    if batch.positions.len() > u32::MAX as usize {
        return Err(EngineError::Gpu("physics batch exceeds GPU index range".to_owned()));
    }
    Ok(())
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuParticle {
    position: [f32; 2],
    velocity: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuParams {
    world_size: [f32; 2],
    count: u32,
    _padding: u32,
}

const PHYSICS_SHADER: &str = r#"
struct Particle {
    position: vec2<f32>,
    velocity: vec2<f32>,
};

struct Params {
    world_size: vec2<f32>,
    count: u32,
    padding: u32,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: Params;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= params.count) {
        return;
    }
    let next = particles[id.x].position + particles[id.x].velocity;
    particles[id.x].position = clamp(next, vec2<f32>(0.0), params.world_size);
}
"#;
