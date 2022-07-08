//Ryan Berg 6/14/22

use wgpu::util::DeviceExt;
use winit::{
    event::{Event , WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
use nanorand::{Rng, WyRand};
use std::{borrow::Cow, mem, slice};
use core_simd::f32x4;

const MAX_AGENT_COUNT:u32 = 10;
const AGENTS_PER_WORK_GROUP: u32 = 64;
const CAP_FRAME_RATE: bool = true;
const USE_DEDICATED_GPU: bool = false;
const ZONE_3_SIZE: u32 = 100;

pub async fn init(window_width: u32, window_height: u32)
{
    //Build WGPU and Shaders
    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("Zone 3")
        .with_inner_size(winit::dpi::PhysicalSize::new(window_width, window_height))
        .build(&event_loop)
        .unwrap();

    // zone_width /= 3;

    let web_gpu = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe {web_gpu.create_surface(&window)};

    let gpu_choice = if USE_DEDICATED_GPU == false {wgpu::PowerPreference::default()} else {wgpu::PowerPreference::HighPerformance};

    let adapter = web_gpu.request_adapter(&wgpu::RequestAdapterOptions{
        power_preference: gpu_choice,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false
    }).await.expect("Could not find adapter.");


    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor{
        label: None,
        features: wgpu::Features::empty(),
        limits: wgpu::Limits::default()
    }, None).await.expect("Could not create device");



    let diffuse_compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Diffuse Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("diffuse.wgsl"))),
    });
    let draw_agent_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Draw Agent Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("draw_agent.wgsl"))),
    });
    let draw_signal_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Draw Signal Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("draw_signal.wgsl"))),
    });
    let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Compute Agent Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("flock.wgsl"))),
    });
    let mut signal_data = vec![0.0f32; (1 * ZONE_3_SIZE * ZONE_3_SIZE) as usize];

    let mut count = 0;
    for signal in signal_data.chunks_mut(1) {
        if count == 0 || count == 5550 {
            signal[0] = 5000.;
        }
        else {
            signal[0] = 0.;
        }
        count += 1;
    }

    //Compute
    let sim_parameters = [
        0.04f32, // deltaT
        0.1,     // rule1Distance
        0.025,   // rule2Distance
        0.025,   // rule3Distance
        0.02,    // rule1Scale
        0.05,    // rule2Scale
        0.005,   // rule3Scale
    ].to_vec();

    let sim_param_buffer = device.create_buffer_init( &wgpu::util::BufferInitDescriptor{
        label: Some("Sim Parameter Buffer"),
        contents: bytemuck::cast_slice(&sim_parameters),
        usage: wgpu::BufferUsages::UNIFORM
    });

    let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0, //sim parameters
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (sim_parameters.len() * mem::size_of::<f32>()) as _,
                    ),
                }
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1, //input agent attributes
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new((MAX_AGENT_COUNT * 16) as _)
                }
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2, //output agent attributes
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new((MAX_AGENT_COUNT * 16) as _)
                },
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3, //input signal values
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (signal_data.len() * mem::size_of::<f32>()) as _,
                    )
                }
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4, //output signal values
                count: None,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (signal_data.len() * mem::size_of::<f32>()) as _,
                    )
                }
            }
        ]
    });

    let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
       label: Some("Compute"),
       bind_group_layouts: &[&compute_bind_group_layout],
       push_constant_ranges: &[]
    });
    
    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
        label: Some("Compute Pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &compute_shader,
        entry_point: "main"
    });

    //Signal Vertex and Fragment

    let mut general_parameters = [
        -1.,    // inner window offset x
        0.,     // inner window offset y
        window_width as f32,   // outer window resolution x
        window_height as f32,   // outer window resolution y
        1066.,    // inner window resolution x
        1066.,    // inner window resolution y
    ];

    let general_param_buffer = device.create_buffer_init( &wgpu::util::BufferInitDescriptor{
        label: Some("Sim Parameter Buffer"),
        contents: bytemuck::cast_slice(&general_parameters),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
    });

    let signal_vertex_buffer_data:[[f32;2]; 6] = [[0., 0.], [1., 0.], [1., 1.], [0., 0.], [1., 0.], [1., 1.]];
    let signal_vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Signal Vertex Buffer"),
        contents: bytemuck::bytes_of(&signal_vertex_buffer_data),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let signal_vertices_buffer2 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Signal Vertex Buffer"),
        contents: bytemuck::bytes_of(&signal_vertex_buffer_data),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let mut signal_buffers = Vec::<wgpu::Buffer>::new();
    for i in 0..2 {
        signal_buffers.push(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Signal Buffer {}", i)),
                contents: bytemuck::cast_slice(&signal_data),
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST,
            }),
        );
    }


    //Signal Draw Bindings
    let signal_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: Some("Signal Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0, //signal strength
                count: None,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (signal_data.len() * mem::size_of::<f32>()) as _,
                    )
                }
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1, //signal strength
                count: None,
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (signal_data.len() * mem::size_of::<f32>()) as _,
                    )
                }
            },
        ]
    });

    let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: Some("Signal Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0, //signal strength
                count: None,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (general_parameters.len() * mem::size_of::<f32>()) as _,
                    )
                }
            },
        ]
    });

    let signal_render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Signal Render"),
        bind_group_layouts: &[&signal_bind_group_layout, &uniform_bind_group_layout],
        push_constant_ranges: &[],
    });

    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
        label: Some("Uniform Buffer"),
        layout: &uniform_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry{
                binding: 0,
                resource: general_param_buffer.as_entire_binding()
            }
        ]
    });

    let signal_bind_group2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(&format!("Singal Buffer2")),
        layout: &signal_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: signal_buffers[0].as_entire_binding()
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: signal_buffers[1].as_entire_binding()
            }
        ]
    });

    let mut signal_bind_groups = Vec::<wgpu::BindGroup>::new();

    for i in 0..2
    {
        signal_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some(&format!("Singal Buffer {}", i)),
            layout: &signal_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: signal_buffers[i].as_entire_binding()
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: signal_buffers[(i+1)%2].as_entire_binding()
                }
            ]
        }));
    }

    let signal_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
        label: None,
        layout: Some(&signal_render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &draw_signal_shader,
            entry_point: "vs_main",
            buffers: &[]
        },
        fragment: Some(wgpu::FragmentState {
            module: &draw_signal_shader,
            entry_point: "fs_main",
            targets: &[Some(
                wgpu::ColorTargetState{
                    format: surface.get_supported_formats(&adapter)[0],
                    blend: Some(wgpu::BlendState{
                        color: wgpu::BlendComponent{
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,},
                        alpha: wgpu::BlendComponent::OVER
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })
            ],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let signal_render_pipeline2 = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
        label: None,
        layout: Some(&signal_render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &draw_signal_shader,
            entry_point: "vs_main",
            buffers: &[]
        },
        fragment: Some(wgpu::FragmentState {
            module: &draw_signal_shader,
            entry_point: "fs_main",
            targets: &[Some(surface_.format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    //Diffuse Compute
    let diffuse_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
        label: Some("Diffuse Pipeline Layout"),
        bind_group_layouts: &[&signal_bind_group_layout],
        push_constant_ranges: &[]
    });

    let diffuse_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
        label: Some("Diffuse Pipeline"),
        layout: Some(&diffuse_pipeline_layout),
        module: &diffuse_compute_shader,
        entry_point: "main"
    });

    //Agent Vertex and Fragment
    let agent_render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Agent Render"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

    let agent_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
        label: None,
        layout: Some(&agent_render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &draw_agent_shader,
            entry_point: "vs_main",
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: 4 * 4,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                },
                wgpu::VertexBufferLayout {
                    array_stride: 2 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![2 => Float32x2],
                }
            ]
        },
        fragment: Some(wgpu::FragmentState {
            module: &draw_agent_shader,
            entry_point: "fs_main",
            targets: &[surface.get_preferred_format(&adapter).unwrap().into()],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    //object shape
    let agent_vertex_buffer_data:[[f32;1]; 6] = [[-0.01f32], [-0.02], [0.01], [-0.02], [0.00], [0.02]];
    let agent_vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Agent Vertex Buffer"),
        contents: bytemuck::bytes_of(&agent_vertex_buffer_data),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    //object associated data
    let mut initial_agent_data = vec![0.0f32; (4 * MAX_AGENT_COUNT) as usize];
    let mut rng = WyRand::new_seed(42);
    let mut unif = || rng.generate::<f32>() * 2f32 - 1f32; // Generate a num (-1, 1)
    for agent_instance_chunk in initial_agent_data.chunks_mut(4) {
        agent_instance_chunk[0] = unif(); // posx
        agent_instance_chunk[1] = unif(); // posy
        agent_instance_chunk[2] = unif() * 0.1; // velx
        agent_instance_chunk[3] = unif() * 0.1; // vely
    }

    let mut agent_buffers = Vec::<wgpu::Buffer>::new();
    let mut agent_bind_groups = Vec::<wgpu::BindGroup>::new();
    for i in 0..2 {
        agent_buffers.push(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Agent Buffer {}", i)),
                contents: bytemuck::cast_slice(&initial_agent_data),
                usage: wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST,
            }),
        );
    }

    //Compute Bindings
    for i in 0..2
    {
        agent_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: None,
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: sim_param_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: agent_buffers[i].as_entire_binding()
                },
                wgpu::BindGroupEntry{
                    binding: 2,
                    resource: agent_buffers[(i+1)%2].as_entire_binding()
                },
                wgpu::BindGroupEntry{
                    binding: 3,
                    resource: signal_buffers[i].as_entire_binding()
                },
                wgpu::BindGroupEntry{
                    binding: 4,
                    resource: signal_buffers[(i+1)%2].as_entire_binding()
                }
            ]
        }))
    }

    //Surface
    let gpu_choice = if CAP_FRAME_RATE == false {wgpu::PresentMode::Mailbox} else {wgpu::PresentMode::Fifo};

    let mut surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_preferred_format(&adapter).unwrap(),
        width: window.inner_size().width,
        height: window.inner_size().height,
        present_mode: wgpu::PresentMode::Mailbox
    };
    surface.configure(&device, &surface_config);


    // calculates number of work groups from PARTICLES_PER_GROUP constant
    let work_group_count = ((MAX_AGENT_COUNT as f32) / (AGENTS_PER_WORK_GROUP as f32)).ceil() as u32;
    let diffuse_work_group_count = (((ZONE_3_SIZE * ZONE_3_SIZE) as f32) / (AGENTS_PER_WORK_GROUP as f32)).ceil() as u32;

    let mut frame_count:usize = 0;
    let start = std::time::Instant::now();
    event_loop.run(move |event, _, control_flow| {
        // Have the closure take ownership of the resources.
        // `event_loop.run` never returns, therefore we must do this to ensure
        // the resources are properly cleaned up.
        let _ = (&web_gpu, &adapter, &draw_agent_shader, &agent_render_pipeline_layout);

        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                // Reconfigure the surface with the new size
                surface_config.width = size.width;
                surface_config.height = size.height;
                surface.configure(&device, &surface_config);
                // On macos the window needs to be redrawn manually after resizing
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let frame = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                general_parameters[0] = -1.;
                queue.write_buffer(&general_param_buffer, 0, bytemuck::cast_slice(&[general_parameters]));

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Command Encoder") });

                {
                    let mut diffuse_signal_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{label:Some("Diffuse Signal Compute Pass")});

                    diffuse_signal_pass.set_pipeline(&diffuse_pipeline);
                    diffuse_signal_pass.set_bind_group(0, &signal_bind_groups[frame_count%2], &[]);
                    diffuse_signal_pass.dispatch_workgroups(diffuse_work_group_count, 1, 1);
                }

                {
                    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{label:Some("Agent Compute Pass")});

                    compute_pass.set_pipeline(&compute_pipeline);
                    compute_pass.set_bind_group(0, &agent_bind_groups[frame_count%2], &[]);
                    compute_pass.dispatch_workgroups(work_group_count, 1, 1);
                }



                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color{r:0., g: 0.5, b: 0.5, a: 1.}),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    {

                        rpass.set_pipeline(&signal_render_pipeline);
                        // render dst agents
                        // rpass.set_vertex_buffer(0, signal_buffers[((frame_count + 1) % 2)].slice(..));
                        // the three instance-local vertices

                        rpass.set_bind_group(0, &signal_bind_groups[0], &[]);
                        rpass.set_bind_group(1, &uniform_bind_group, &[]);
                        rpass.set_vertex_buffer(0, signal_vertices_buffer.slice(..));
                        // rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                        // rpass.draw_indexed(0..num_indices, 0, 0..1);
                        rpass.draw(0..6, 0..1);

                    }



                }

                queue.submit(Some(encoder.finish()));

                general_parameters[0] = -0.334;
                queue.write_buffer(&general_param_buffer, 0, bytemuck::cast_slice(&[general_parameters]));

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Command Encoder") });

                {
                    let mut diffuse_signal_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{label:Some("Diffuse Signal Compute Pass")});

                    diffuse_signal_pass.set_pipeline(&diffuse_pipeline);
                    diffuse_signal_pass.set_bind_group(0, &signal_bind_groups[frame_count%2], &[]);
                    diffuse_signal_pass.dispatch_workgroups(diffuse_work_group_count, 1, 1);
                }

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    {
                        rpass.set_pipeline(&signal_render_pipeline);
                        rpass.set_bind_group(0, &signal_bind_groups[0], &[]);
                        rpass.set_bind_group(1, &uniform_bind_group, &[]);
                        rpass.set_vertex_buffer(0, signal_vertices_buffer.slice(..));
                        rpass.draw(0..6, 0..1);
                    }
                }
                queue.submit(Some(encoder.finish()));

                general_parameters[0] = 0.334;
                queue.write_buffer(&general_param_buffer, 0, bytemuck::cast_slice(&[general_parameters]));

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Command Encoder") });

                {
                    let mut diffuse_signal_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{label:Some("Diffuse Signal Compute Pass")});

                    diffuse_signal_pass.set_pipeline(&diffuse_pipeline);
                    diffuse_signal_pass.set_bind_group(0, &signal_bind_groups[frame_count%2], &[]);
                    diffuse_signal_pass.dispatch_workgroups(diffuse_work_group_count, 1, 1);
                }

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    {
                        rpass.set_pipeline(&signal_render_pipeline);
                        rpass.set_bind_group(0, &signal_bind_groups[0], &[]);
                        rpass.set_bind_group(1, &uniform_bind_group, &[]);
                        rpass.set_vertex_buffer(0, signal_vertices_buffer.slice(..));
                        rpass.draw(0..6, 0..1);
                    }

                    {
                        rpass.set_pipeline(&agent_render_pipeline);
                        // render dst agents
                        rpass.set_vertex_buffer(0, agent_buffers[((frame_count + 1) % 2)].slice(..));
                        // the three instance-local vertices
                        rpass.set_vertex_buffer(1, agent_vertices_buffer.slice(..));
                        rpass.draw(0..3, 0..MAX_AGENT_COUNT);
                    }
                }
                queue.submit(Some(encoder.finish()));


                frame.present();
                frame_count += 1;
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}
const horizontal_square_count:u32 = 3;
const number_of_vertices:u32 = horizontal_square_count * 6 * horizontal_square_count;
const buffer_length:usize = (number_of_vertices * 2) as usize;
fn build_plane(width: u32) -> [f32; buffer_length]
{
    let square_size = 1.9 / horizontal_square_count as f32;
    let mut vertex_array: [f32; buffer_length] = [0.; buffer_length];
    let mut vertex_count = 0;
    for row in 0..horizontal_square_count{
        for column in 0..horizontal_square_count{
            let origin_x =  column as f32*square_size; let origin_y = row as f32*square_size;

            if (row + column) as f32 % 2. == 0. {
                vertex_array[vertex_count] = origin_x; vertex_array[vertex_count+1] = origin_y;
                vertex_array[vertex_count+2] = origin_x; vertex_array[vertex_count+3] = origin_y + square_size;
                vertex_array[vertex_count+4] = origin_x + square_size; vertex_array[vertex_count+5] = origin_y;
                vertex_array[vertex_count+6] = origin_x; vertex_array[vertex_count+7] = origin_y + square_size;
                vertex_array[vertex_count+8] = origin_x + square_size; vertex_array[vertex_count+9] = origin_y;
                vertex_array[vertex_count+10] = origin_x + square_size; vertex_array[vertex_count+11] = origin_y + square_size;
            }
            else {
                vertex_array[vertex_count] = origin_x; vertex_array[vertex_count+1] = origin_y + square_size;
                vertex_array[vertex_count+2] = origin_x; vertex_array[vertex_count+3] = origin_y;
                vertex_array[vertex_count+4] = origin_x + square_size; vertex_array[vertex_count+5] = origin_y + square_size;
                vertex_array[vertex_count+6] = origin_x; vertex_array[vertex_count+7] = origin_y;
                vertex_array[vertex_count+8] = origin_x + square_size; vertex_array[vertex_count+9] = origin_y + square_size;
                vertex_array[vertex_count+10] = origin_x + square_size; vertex_array[vertex_count+11] = origin_y;
            }
            vertex_count += 12;
        }
    }
    return vertex_array;
}
fn random(min:f32, max:f32, seed:u64) ->f32{
    let prand = WyRand::new_seed(seed).generate::<f32>();
    let range = max - min;
    return prand * range - min; // Generate a num (-1, 1)
}

















