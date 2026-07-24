//! Native, GPU-accelerated resolution-independent vector rendering
//! (spec.txt §2.1 "Vector Graphics"): cubic Bezier paths are tessellated
//! on the GPU using a compute shader, never on the CPU.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// A cubic Bezier curve segment in normalized device coordinates.
#[derive(Debug, Clone, Copy)]
pub struct CubicBezier {
    pub p0: [f32; 2],
    pub p1: [f32; 2],
    pub p2: [f32; 2],
    pub p3: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct CubicBezierGpu {
    p0: [f32; 2],
    p1: [f32; 2],
    p2: [f32; 2],
    p3: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct TessellateParams {
    segments_per_curve: u32,
    curve_count: u32,
    _pad0: u32,
    _pad1: u32,
}

/// Builds the four cubic Beziers that approximate a full circle of the
/// given radius centered at `center`, using the standard k ~= 0.5522847498
/// control-point offset.
pub fn circle_path(center: [f32; 2], radius: f32) -> Vec<CubicBezier> {
    const K: f32 = 0.552_284_8;
    let (cx, cy) = (center[0], center[1]);
    let r = radius;
    let rk = radius * K;

    let pt = |x: f32, y: f32| [cx + x, cy + y];

    vec![
        CubicBezier {
            p0: pt(r, 0.0),
            p1: pt(r, rk),
            p2: pt(rk, r),
            p3: pt(0.0, r),
        },
        CubicBezier {
            p0: pt(0.0, r),
            p1: pt(-rk, r),
            p2: pt(-r, rk),
            p3: pt(-r, 0.0),
        },
        CubicBezier {
            p0: pt(-r, 0.0),
            p1: pt(-r, -rk),
            p2: pt(-rk, -r),
            p3: pt(0.0, -r),
        },
        CubicBezier {
            p0: pt(0.0, -r),
            p1: pt(rk, -r),
            p2: pt(r, -rk),
            p3: pt(r, 0.0),
        },
    ]
}

/// GPU-accelerated cubic Bezier tessellation: dispatches a compute shader
/// that evaluates De Casteljau's formula for every sample point in
/// parallel, then reads the resulting points back.
///
/// Returns `curves.len() * (segments_per_curve + 1)` points, in order,
/// each curve's points contiguous.
pub fn tessellate_cubics_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    curves: &[CubicBezier],
    segments_per_curve: u32,
) -> Vec<[f32; 2]> {
    let curve_count = curves.len() as u32;
    let points_per_curve = segments_per_curve + 1;
    let total_points = (points_per_curve * curve_count) as usize;

    let curves_gpu: Vec<CubicBezierGpu> = curves
        .iter()
        .map(|c| CubicBezierGpu {
            p0: c.p0,
            p1: c.p1,
            p2: c.p2,
            p3: c.p3,
        })
        .collect();

    let params = TessellateParams {
        segments_per_curve,
        curve_count,
        _pad0: 0,
        _pad1: 0,
    };

    let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("chora-vector-tessellate-params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let curves_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("chora-vector-tessellate-curves"),
        contents: bytemuck::cast_slice(&curves_gpu),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let output_size = (total_points * std::mem::size_of::<[f32; 2]>()) as u64;
    let output_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("chora-vector-tessellate-output"),
        size: output_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let staging_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("chora-vector-tessellate-staging"),
        size: output_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("chora-vector-tessellate-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/tessellate.wgsl").into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("chora-vector-tessellate-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("chora-vector-tessellate-bg"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: curves_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: output_buf.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("chora-vector-tessellate-pl"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("chora-vector-tessellate-pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "tessellate",
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("chora-vector-tessellate-encoder"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("chora-vector-tessellate-pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        let workgroups = (total_points as u32).div_ceil(64).max(1);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size);
    queue.submit(std::iter::once(encoder.finish()));

    let slice = staging_buf.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .expect("map_async callback dropped without a result")
        .expect("failed to map tessellation readback buffer");

    let data = slice.get_mapped_range();
    let points: Vec<[f32; 2]> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    staging_buf.unmap();

    points
}
