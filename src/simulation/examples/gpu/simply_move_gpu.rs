// src/simulation/examples/gpu/simply_move_gpu.rs - Simplified with wgpu utils
//! Simplified GPU simulation using enhanced wgpu utilities

use crate::{
    gfx::scene::Scene,
    simulation::traits::Simulation,
    wgpu_utils::{
        binding_builder::{BindGroupBuilder, BindGroupLayoutBuilder},
        binding_types,
        uniform_buffer::{ArrayBuffer, UniformBuffer},
    },
};
use imgui::Ui;
use wgpu::{BindGroup, ComputePipeline, Device, Queue};

/// GPU data structure for object transforms
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ObjectTransform {
    position: [f32; 3],
    _padding1: f32,
    rotation: [f32; 3],
    _padding2: f32,
    scale: [f32; 3],
    _padding3: f32,
}

/// Uniforms passed to the compute shader
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct SimulationUniforms {
    time: f32,
    delta_time: f32,
    amplitude: f32,
    frequency: f32,
    object_count: u32,
    _padding: [u32; 3],
}

/// Simplified GPU simulation that moves all objects up and down
pub struct GpuSimplyMove {
    running: bool,
    time: f32,
    amplitude: f32,
    frequency: f32,

    // GPU resources - simplified with utils
    gpu_ready: bool,
    compute_pipeline: Option<ComputePipeline>,
    bind_group: Option<BindGroup>,

    // Using our enhanced buffer utilities
    transform_buffer: Option<ArrayBuffer<ObjectTransform>>,
    uniform_buffer: Option<UniformBuffer<SimulationUniforms>>,
    initial_positions_buffer: Option<ArrayBuffer<f32>>,
    staging_buffer: Option<ArrayBuffer<ObjectTransform>>,

    initial_positions: Vec<f32>,
    object_count: u32,
}

impl GpuSimplyMove {
    pub fn new() -> Self {
        Self {
            running: true,
            time: 0.0,
            amplitude: 2.0,
            frequency: 0.1,
            gpu_ready: false,
            compute_pipeline: None,
            bind_group: None,
            transform_buffer: None,
            uniform_buffer: None,
            initial_positions_buffer: None,
            staging_buffer: None,
            initial_positions: Vec::new(),
            object_count: 0,
        }
    }

    fn create_shader_source() -> &'static str {
        r#"
struct ObjectTransform {
    position: vec3<f32>,
    rotation: vec3<f32>,
    scale: vec3<f32>,
}

struct Uniforms {
    time: f32,
    delta_time: f32,
    amplitude: f32,
    frequency: f32,
    object_count: u32,
}

@group(0) @binding(0) var<storage, read_write> transforms: array<ObjectTransform>;
@group(0) @binding(1) var<uniform> uniforms: Uniforms;
@group(0) @binding(2) var<storage, read> initial_positions: array<f32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let index = global_id.x;
    if (index >= uniforms.object_count) {
        return;
    }
    
    // Calculate sine wave offset
    let phase = uniforms.time * uniforms.frequency * 2.0 * 3.14159;
    let y_offset = sin(phase) * uniforms.amplitude;
    
    // Update position
    transforms[index].position.y = initial_positions[index] + y_offset;
    
    // Update rotation for some visual flair
    transforms[index].rotation.y = uniforms.time * 45.0;
}
"#
    }
}

impl Simulation for GpuSimplyMove {
    fn initialize(&mut self, scene: &mut Scene) {
        println!("Initializing GPU SimplyMove simulation...");

        // Store initial Y positions
        self.initial_positions.clear();
        self.object_count = 0;

        for object in &scene.objects {
            if !object.name.starts_with('_') {
                self.initial_positions.push(object.ui_transform.position[1]);
                self.object_count += 1;
            }
        }

        println!("Found {} objects for GPU simulation", self.object_count);
    }

    fn update(&mut self, delta_time: f32, _scene: &mut Scene) {
        if !self.running {
            return;
        }

        self.time += delta_time;
        // All movement happens on GPU via update_gpu()
    }

    fn render_ui(&mut self, ui: &Ui) {
        let display_size = ui.io().display_size;
        let panel_width = 300.0;
        let panel_x = display_size[0] - panel_width - 20.0;

        ui.window("GPU SimplyMove Controls")
            .size([panel_width, 220.0], imgui::Condition::FirstUseEver)
            .position([panel_x, 20.0], imgui::Condition::FirstUseEver)
            .build(|| {
                ui.text("GPU Compute Shader Movement");

                if self.gpu_ready {
                    ui.text_colored([0.0, 1.0, 0.0, 1.0], "🔹 GPU Active");
                } else {
                    ui.text_colored([1.0, 0.0, 0.0, 1.0], "❌ GPU Not Ready");
                }

                ui.separator();

                ui.slider("Amplitude", 0.1, 5.0, &mut self.amplitude);
                ui.slider("Frequency", 0.1, 3.0, &mut self.frequency);

                ui.separator();

                ui.text(&format!("Time: {:.2}s", self.time));
                ui.text(&format!("Objects: {}", self.object_count));

                if self.gpu_ready {
                    let y_offset = (self.time * self.frequency * 2.0 * std::f32::consts::PI).sin()
                        * self.amplitude;
                    ui.text(&format!("Y Offset: {:.2}", y_offset));
                }

                ui.separator();

                if ui.button("Reset Time") {
                    self.time = 0.0;
                }
            });
    }

    fn name(&self) -> &str {
        "GPU SimplyMove"
    }

    fn is_running(&self) -> bool {
        self.running
    }

    fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    fn reset(&mut self, scene: &mut Scene) {
        self.time = 0.0;

        // Reset to initial positions
        let mut obj_index = 0;
        for object in scene.objects.iter_mut() {
            if object.name.starts_with('_') {
                continue;
            }

            if let Some(initial_y) = self.initial_positions.get(obj_index) {
                object.ui_transform.position[1] = *initial_y;
                object.ui_transform.rotation[1] = 0.0;
                object.apply_ui_transform();
            }
            obj_index += 1;
        }
    }

    fn cleanup(&mut self, scene: &mut Scene) {
        self.reset(scene);
        self.gpu_ready = false;
        self.compute_pipeline = None;
        self.bind_group = None;
        self.transform_buffer = None;
        self.uniform_buffer = None;
        self.initial_positions_buffer = None;
        self.staging_buffer = None;
    }

    // GPU-specific methods - now much cleaner!
    fn initialize_gpu(&mut self, device: &Device, _queue: &Queue) {
        if self.object_count == 0 {
            println!("No objects to simulate on GPU");
            return;
        }

        println!(
            "Initializing GPU resources for {} objects",
            self.object_count
        );

        // Create buffers using our enhanced utilities
        let transform_buffer = ArrayBuffer::new(device, self.object_count as usize, false);
        let uniform_buffer = UniformBuffer::new(device);
        let initial_positions_buffer =
            ArrayBuffer::new_with_data(device, &self.initial_positions, true);
        let staging_buffer = ArrayBuffer::new_staging(device, self.object_count as usize); // Use staging buffer

        // Create bind group layout using the builder
        let layout_with_desc = BindGroupLayoutBuilder::new()
            .next_binding_compute(binding_types::storage_buffer_read_write()) // transforms
            .next_binding_compute(binding_types::uniform()) // uniforms
            .next_binding_compute(binding_types::storage_buffer_read_only()) // initial_positions
            .create(device, "GPU SimplyMove Layout");

        // Create bind group using the builder
        let bind_group = BindGroupBuilder::new(&layout_with_desc)
            .buffer(transform_buffer.buffer())
            .buffer(uniform_buffer.buffer())
            .buffer(initial_positions_buffer.buffer())
            .create(device, "GPU SimplyMove Bind Group");

        // Create compute pipeline
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GPU SimplyMove Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::create_shader_source().into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GPU SimplyMove Pipeline Layout"),
            bind_group_layouts: &[&layout_with_desc.layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("GPU SimplyMove Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Store resources
        self.transform_buffer = Some(transform_buffer);
        self.uniform_buffer = Some(uniform_buffer);
        self.initial_positions_buffer = Some(initial_positions_buffer);
        self.staging_buffer = Some(staging_buffer);
        self.bind_group = Some(bind_group);
        self.compute_pipeline = Some(compute_pipeline);
        self.gpu_ready = true;

        println!("GPU resources initialized successfully");
    }

    fn update_gpu(&mut self, device: &Device, queue: &Queue, delta_time: f32) {
        if !self.gpu_ready || self.object_count == 0 {
            return;
        }

        let Some(pipeline) = &self.compute_pipeline else {
            return;
        };
        let Some(bind_group) = &self.bind_group else {
            return;
        };
        let Some(uniform_buffer) = &mut self.uniform_buffer else {
            return;
        };
        let Some(transform_buffer) = &self.transform_buffer else {
            return;
        };
        let Some(staging_buffer) = &self.staging_buffer else {
            return;
        };

        // Update uniforms using the enhanced buffer
        let uniforms = SimulationUniforms {
            time: self.time,
            delta_time,
            amplitude: self.amplitude,
            frequency: self.frequency,
            object_count: self.object_count,
            _padding: [0; 3],
        };

        uniform_buffer.update_content(queue, uniforms);

        // Dispatch compute shader
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GPU SimplyMove Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GPU SimplyMove Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);

            let workgroups = (self.object_count + 63) / 64;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Copy results to staging buffer for readback
        encoder.copy_buffer_to_buffer(
            transform_buffer.buffer(),
            0,
            staging_buffer.buffer(),
            0,
            (self.object_count as u64) * std::mem::size_of::<ObjectTransform>() as u64,
        );

        queue.submit(std::iter::once(encoder.finish()));
    }

    fn apply_gpu_results_to_scene(&mut self, device: &Device, scene: &mut Scene) {
        if !self.gpu_ready || self.object_count == 0 {
            return;
        }

        let Some(staging_buffer) = &self.staging_buffer else {
            return;
        };

        // Map the staging buffer and read the data
        let buffer_slice = staging_buffer.buffer().slice(..);

        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });

        let _ = device.poll(wgpu::MaintainBase::Wait);

        if let Ok(Ok(())) = receiver.recv() {
            let data = buffer_slice.get_mapped_range();
            let transforms: &[ObjectTransform] = bytemuck::cast_slice(&data);

            // Apply transforms to scene objects
            let mut obj_index = 0;
            for object in scene.objects.iter_mut() {
                if object.name.starts_with('_') {
                    continue;
                }

                if let Some(transform) = transforms.get(obj_index) {
                    object.ui_transform.position = transform.position;
                    object.ui_transform.rotation = transform.rotation;
                    object.apply_ui_transform();
                }

                obj_index += 1;
                if obj_index >= self.object_count as usize {
                    break;
                }
            }

            drop(data);
            staging_buffer.buffer().unmap();
        }
    }

    fn is_gpu_ready(&self) -> bool {
        self.gpu_ready
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
