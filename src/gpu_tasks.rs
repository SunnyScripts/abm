//Created by Ryan Berg 7/5/22
//ToDo: use async functions

use wgpu::{Extent3d};
use std::mem;
use imgui::TextureId;
use wgpu::util::DeviceExt;
use crate::{Shaders, UniformData};
use imgui_wgpu::Renderer;
use bytemuck::cast_slice;
use rand::distributions::{Distribution, Uniform};

const PORTAL: i32 = 0;
const AGENT_WANDER: i32 = 1;
const AGENT_CHASE: i32 = 2;

pub struct GPUTasks {
    pub uniform_buffer: wgpu::Buffer,
    global_bind_group: wgpu::BindGroup,

    clear_agent_buffers_compute_pipeline: wgpu::ComputePipeline,
    clear_agent_buffers_bind_groups: Vec<wgpu::BindGroup>,

    agent_compute_pipeline: wgpu::ComputePipeline,
    agent_list_bind_groups: Vec<wgpu::BindGroup>,
    agent_texture_bind_groups: Vec<wgpu::BindGroup>,

    compute_diffuse_pipeline: wgpu::ComputePipeline,
    diffuse_compute_bind_groups: Vec<wgpu::BindGroup>,

    render_pipeline: wgpu::RenderPipeline,
    render_bind_groups: Vec<Vec<wgpu::BindGroup>>,

    active_frame: bool,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,
    instance_count: u32,
}

impl GPUTasks{
    //init
    pub fn init(uniform_buffer_data: UniformData, shader: Shaders, texture_format: wgpu::TextureFormat, device: &wgpu::Device, queue: &wgpu::Queue) -> Self{

        let vertex_buffer_data = vec![0., 0., 1., 0., 1., 1., 0., 0., 1., 0., 1., 1.];
        let empty_texture_data = vec![0u8; (4. * uniform_buffer_data.zone1_dimensions[0] * uniform_buffer_data.zone1_dimensions[1]) as usize];

        let current_agent_count = 99i32;
        let mut agent_list = vec![0f32; 5 * current_agent_count as usize];

        let mut agent_grid_occupancy_data = vec![];
        agent_grid_occupancy_data.push(vec![-1i32; (3. * uniform_buffer_data.zone1_dimensions[0] * uniform_buffer_data.zone1_dimensions[1]) as usize]);
        agent_grid_occupancy_data.push(vec![-1i32; (3. * uniform_buffer_data.zone2_dimensions[0] * uniform_buffer_data.zone2_dimensions[1])  as usize]);
        agent_grid_occupancy_data.push(vec![-1i32; (3. * uniform_buffer_data.zone3_dimensions[0] * uniform_buffer_data.zone3_dimensions[1]) as usize]);

        let mut rng = rand::thread_rng();
        let space_range = Uniform::from(0..100);
        let zone_range = Uniform::from(1..=3);

        for i in 0..6 {
            let mut x = space_range.sample(&mut rng);
            let mut y = space_range.sample(&mut rng);
            let mut zone = 1f32;
            if i > 2 {zone = 3f32;}

            agent_list[(i * 10) as usize] = PORTAL as f32;      //state
            agent_list[(i * 10) as usize + 1] = zone as f32;    //zone
            agent_list[(i * 10) as usize + 2] = x as f32;       //x pos
            agent_list[(i * 10) as usize + 3] = y as f32;       //y pos
            agent_list[(i * 10) as usize + 4] = (i*2+1) as f32; //opposite portal index

            let grid_index = y * uniform_buffer_data.zone1_dimensions[1] as i32 + x;
            agent_grid_occupancy_data[(zone - 1.) as usize][(grid_index * 3) as usize] = i*2;


            x = space_range.sample(&mut rng);
            y = space_range.sample(&mut rng);

            agent_list[(i * 10) as usize + 5] = PORTAL as f32;   //state
            agent_list[(i * 10) as usize + 6] = 2.;              //zone
            agent_list[(i * 10) as usize + 7] = x as f32;        //x pos
            agent_list[(i * 10) as usize + 8] = y as f32;        //y pos
            agent_list[(i * 10) as usize + 9] = (i*2) as f32;    //opposite portal index

            let grid_index = y * uniform_buffer_data.zone1_dimensions[1] as i32 + x;
            agent_grid_occupancy_data[1][(grid_index * 3) as usize] = i*2+1
        }

        for i in 12..current_agent_count {
            agent_list[(i * 5) as usize] = AGENT_WANDER as f32;                             //state
            agent_list[(i * 5) as usize + 1] = zone_range.sample(&mut rng) as f32;          //zone
            agent_list[(i * 5) as usize + 2] = space_range.sample(&mut rng) as f32;         //x pos
            agent_list[(i * 5) as usize + 3] = space_range.sample(&mut rng) as f32;         //y pos
            agent_list[(i * 5) as usize + 4] = (1+i) as f32 / current_agent_count as f32;   //unique ID
        }

        create_objects(6, 1,
                       shader,
                       vertex_buffer_data, uniform_buffer_data, empty_texture_data,
                       agent_list, agent_grid_occupancy_data,
                       texture_format, device, queue)
    }

    //Compute Pass
    pub fn compute_pass(&mut self, queue: &wgpu::Queue, device: &wgpu::Device)
    {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Command Encoder") });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Clear Agent Buffers Compute Pass") });

            compute_pass.set_pipeline(&self.clear_agent_buffers_compute_pipeline);
            compute_pass.set_bind_group(0, &self.clear_agent_buffers_bind_groups[self.active_frame as u32 as usize], &[]);
            compute_pass.set_bind_group(1, &self.agent_list_bind_groups[self.active_frame as u32 as usize], &[]);
            compute_pass.dispatch_workgroups(99, 1, 1);
        }
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Agent Compute Pass") });

            compute_pass.set_pipeline(&self.agent_compute_pipeline);
            compute_pass.set_bind_group(0, &self.global_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.diffuse_compute_bind_groups[((self.active_frame as u32 + 1) % 2) as usize], &[]);
            compute_pass.set_bind_group(2, &self.agent_list_bind_groups[self.active_frame as u32 as usize], &[]);
            compute_pass.set_bind_group(3, &self.agent_texture_bind_groups[self.active_frame as u32 as usize], &[]);
            compute_pass.dispatch_workgroups(99, 1, 1);
        }
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Diffuse Compute Pass") });

            compute_pass.set_pipeline(&self.compute_diffuse_pipeline);
            compute_pass.set_bind_group(0, &self.global_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.diffuse_compute_bind_groups[self.active_frame as u32 as usize], &[]);
            compute_pass.dispatch_workgroups(100, 100, 3);
        }
        queue.submit(Some(encoder.finish()));
        self.toggle_active_frame();
    }

    pub fn draw(&mut self, texture_ids: [Option<TextureId>; 3], renderer: &mut Renderer, queue: &wgpu::Queue, device: &wgpu::Device){

        for (i, texture_id) in texture_ids.iter().enumerate(){
            match texture_id {
                Some(texture_id) => {

                    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Command Encoder") });
                    {
                        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Render Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &renderer.textures.get(*texture_id).unwrap().view(),
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color{r:0., b:0., g:0., a:0.}),
                                    store: true,
                                },
                            })],
                            depth_stencil_attachment: None,
                        });

                        render_pass.set_pipeline(&self.render_pipeline);
                        render_pass.set_bind_group(0, &self.global_bind_group, &[]);
                        render_pass.set_bind_group(1, &self.render_bind_groups[i][self.active_frame as u32 as usize], &[]);
                        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                        render_pass.draw(0..self.vertex_count, 0..self.instance_count);
                    }
                    queue.submit(Some(encoder.finish()));

                }
                None => {/*none*/}
            }
        }
    }

    //Toggle Active Frame
    pub fn toggle_active_frame(&mut self) {
        if self.active_frame { self.active_frame = false }
        else { self.active_frame = true }
    }
}
//Create Objects
fn create_objects(vertex_count: u32, instance_count: u32,
                  shader: Shaders,
                  vertex_buffer_data: Vec<f32>, uniform_buffer_data: UniformData, empty_texture_data: Vec<u8>, //agent_count: u32,
                  agent_list: Vec<f32>, agent_grid_occupancy_data: Vec<Vec<i32>>,
                  texture_format: wgpu::TextureFormat,
                  device: &wgpu::Device, queue: &wgpu::Queue) -> GPUTasks
{
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("alt uniform buffer"),
        contents: bytemuck::cast_slice(&[uniform_buffer_data]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });


    let (_agent_list_buffers, agent_list_bind_group_layout, agent_list_bind_groups) = build_bind_group(
        device, "Agent List", bytemuck::cast_slice(&agent_list), 2, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        2, wgpu::ShaderStages::COMPUTE, true,
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(
                (agent_list.len() * mem::size_of::<u32>()) as _,
            )
        });

    let mut agent_list_storage_buffers: Vec<wgpu::Buffer> = vec![];
    for i in 0..2 {
        agent_list_storage_buffers.push(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Storage Buffer"),
                contents: bytemuck::cast_slice(&agent_list),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC
            })
        );
    }

    let empty_storage_buffer_data = vec![-1; 3 * (uniform_buffer_data.zone1_dimensions[0] * uniform_buffer_data.zone1_dimensions[1]) as usize];
    let mut agent_grid_storage_buffers: Vec<wgpu::Buffer> = vec![];
    for i in 0..6 {
        agent_grid_storage_buffers.push(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Storage Buffer"),
                //ToDo:
                contents: cast_slice(&agent_grid_occupancy_data[i/2]),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC
            })
        );
    }

    let mut agent_storage_buffer_bind_group_layout_entries: Vec<wgpu::BindGroupLayoutEntry> = vec![];
    for i in 0..4 {
        let mut vector_length = agent_list.len();
        if i > 1 { vector_length = empty_storage_buffer_data.len(); }

        agent_storage_buffer_bind_group_layout_entries.push(
            wgpu::BindGroupLayoutEntry {
                binding: i*2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (vector_length * mem::size_of::<i32>()) as _,
                    ),
                },
                count: None,
            }
        );
        agent_storage_buffer_bind_group_layout_entries.push(
            wgpu::BindGroupLayoutEntry {
                binding: i*2+1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (vector_length * mem::size_of::<i32>()) as _,
                    ),
                },
                count: None,
            }
        );
    }

    let agent_storage_buffer_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: Some("Agent Storage Buffer Bind Group Layout"),
        entries: &*agent_storage_buffer_bind_group_layout_entries,
    });

    let mut agent_storage_buffer_bind_groups: Vec<wgpu::BindGroup> = vec![];
    for i in 0..2 {
        agent_storage_buffer_bind_groups.push(
            device.create_bind_group(&wgpu::BindGroupDescriptor{
                label: None,
                layout: &agent_storage_buffer_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry{
                        binding: 0,
                        resource: agent_list_storage_buffers[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry{
                        binding: 1,
                        resource: agent_list_storage_buffers[(i + 1) % 2].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry{
                        binding: 2,
                        resource: agent_grid_storage_buffers[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry{
                        binding: 3,
                        resource: agent_grid_storage_buffers[(i + 1) % 2].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry{
                        binding: 4,
                        resource:agent_grid_storage_buffers[i + 2].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry{
                        binding: 5,
                        resource: agent_grid_storage_buffers[(i + 1) % 2 + 2].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry{
                        binding: 6,
                        resource: agent_grid_storage_buffers[i + 4].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry{
                        binding: 7,
                        resource: agent_grid_storage_buffers[(i + 1) % 2 + 4].as_entire_binding(),
                    },
                ]
            })
        );
    }

    let mut clear_agent_buffer_bind_group_layout_entries: Vec<wgpu::BindGroupLayoutEntry> = vec![];
    for i in 0..3 {
        clear_agent_buffer_bind_group_layout_entries.push(
            wgpu::BindGroupLayoutEntry {
                binding: i*2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (empty_storage_buffer_data.len() * mem::size_of::<i32>()) as _,
                    ),
                },
                count: None,
            }
        );
        clear_agent_buffer_bind_group_layout_entries.push(
            wgpu::BindGroupLayoutEntry {
                binding: i*2+1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: Default::default()
                },
                count: None
            }
        );
    }

    let clear_agent_buffers_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: Some("Clear Agent Buffers Bind Group Layout"),
        entries: &*clear_agent_buffer_bind_group_layout_entries,
    });

    let texture_size = Extent3d {
        width: uniform_buffer_data.zone1_dimensions[0] as u32,
        height: uniform_buffer_data.zone1_dimensions[1] as u32,
        depth_or_array_layers: 1
    };

    let mut textures: Vec<wgpu::Texture> = vec![];
    let mut texture_views: Vec<wgpu::TextureView> = vec![];
    for i in 0..9{
        textures.push(
            device.create_texture(&wgpu::TextureDescriptor{
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                label: Some("Texture")
            })
        );

        queue.write_texture(
            textures[i].as_image_copy(),
            &empty_texture_data,
            wgpu::ImageDataLayout{
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(uniform_buffer_data.zone1_dimensions[0] as u32 * mem::size_of::<u32>() as u32),
                rows_per_image: std::num::NonZeroU32::new(uniform_buffer_data.zone1_dimensions[1] as u32)
            },
            texture_size
        );
        texture_views.push(
            textures[i].create_view(&wgpu::TextureViewDescriptor::default())
        );
    }

    let mut compute_bind_group_layout_entries: Vec<wgpu::BindGroupLayoutEntry> = vec![];
    for i in 0..3{
        compute_bind_group_layout_entries.push(
            wgpu::BindGroupLayoutEntry{
                binding: i*2, //zone 1 read texture
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float {filterable: true}
                },
                count: None
            },
        );
        compute_bind_group_layout_entries.push(
            wgpu::BindGroupLayoutEntry{
                binding: i*2+1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: Default::default()
                },
                count: None
            },
        );
    }

    let compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: Some("Agent Compute Bind Group Layout"),
        entries: &*compute_bind_group_layout_entries,
    });


    let nearest_sampler = device.create_sampler(&wgpu::SamplerDescriptor{
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let linear_sampler = device.create_sampler(&wgpu::SamplerDescriptor{
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let render_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry{
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None
            },
            wgpu::BindGroupLayoutEntry{
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None
            },
            wgpu::BindGroupLayoutEntry{
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float {filterable: true}
                },
                count: None
            },
            wgpu::BindGroupLayoutEntry{
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float {filterable: true}
                },
                count: None
            },
        ]
    });

    let mut agent_compute_bind_group_layout_entries: Vec<wgpu::BindGroupLayoutEntry> = vec![];
    for i in 0..3{
        agent_compute_bind_group_layout_entries.push(
            wgpu::BindGroupLayoutEntry{
                binding: i*3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: Default::default()
                },
                count: None
            },
        );
        agent_compute_bind_group_layout_entries.push(
            wgpu::BindGroupLayoutEntry {
                binding: i*3+1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (empty_storage_buffer_data.len() * mem::size_of::<i32>()) as _,
                    ),
                },
                count: None,
            }
        );
        agent_compute_bind_group_layout_entries.push(
            wgpu::BindGroupLayoutEntry {
                binding: i*3+2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        (empty_storage_buffer_data.len() * mem::size_of::<i32>()) as _,
                    ),
                },
                count: None,
            },
        );
    }

    let agent_compute_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: Some("Agent Compute Bind Group Layout"),
        entries: &*agent_compute_bind_group_layout_entries,
    });


    let mut agent_texture_bind_groups: Vec<wgpu::BindGroup> = vec![];
    for i in 0..2 {
        agent_texture_bind_groups.push(
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Diffuse Compute Bind Group"),
                layout: &agent_compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_views[0])
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: agent_grid_storage_buffers[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: agent_grid_storage_buffers[(i + 1) % 2].as_entire_binding()
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&texture_views[1])
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: agent_grid_storage_buffers[i + 2].as_entire_binding()
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: agent_grid_storage_buffers[(i + 1) % 2 + 2].as_entire_binding()
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&texture_views[2])
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: agent_grid_storage_buffers[i + 4].as_entire_binding()
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: agent_grid_storage_buffers[(i + 1) % 2 + 4].as_entire_binding()
                    },
                ]
            })
        );
    }

    let mut diffuse_compute_bind_groups: Vec<wgpu::BindGroup> = vec![];
    for i in 0..2 {
        diffuse_compute_bind_groups.push(
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Diffuse Compute Bind Group"),
                layout: &compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_views[i+3])
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&texture_views[(i + 1) % 2 + 3])
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&texture_views[i+5])
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&texture_views[(i + 1) % 2 + 5])
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&texture_views[i+7])
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&texture_views[(i + 1) % 2 + 7])
                    },
                ]
            })
        );
    }
    
    let mut render_bind_groups: Vec<Vec<wgpu::BindGroup>> = vec![];
    for i in 0..3 {
        let mut render_bind_group_zone: Vec<wgpu::BindGroup> = vec![];
        for j in 0..2 {
            render_bind_group_zone.push(
                device.create_bind_group(&wgpu::BindGroupDescriptor{
                    label: None,
                    layout: &render_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry{
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&linear_sampler)
                        },
                        wgpu::BindGroupEntry{
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&nearest_sampler)
                        },
                        wgpu::BindGroupEntry{
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&texture_views[i])
                        },
                        wgpu::BindGroupEntry{
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&texture_views[j+3+(i*2)])
                        },
                    ]
                })
            );
        }
        render_bind_groups.push(render_bind_group_zone);
    }

    let global_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: Some("Global Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry{
                binding: 0, //zone 1 read texture
                visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None
            },
        ]
    });

    let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
        label: None,
        layout: &global_layout,
        entries: &[
            wgpu::BindGroupEntry{
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
        ]
    });

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&vertex_buffer_data),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let mut clear_agent_buffers_bind_groups: Vec<wgpu::BindGroup> = vec![];
    for i in 0..2 {
        clear_agent_buffers_bind_groups.push(
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Clear Agent Buffer Bind Group"),
                layout: &clear_agent_buffers_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: agent_grid_storage_buffers[0].as_entire_binding()
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&texture_views[0])
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: agent_grid_storage_buffers[2].as_entire_binding()
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&texture_views[1])
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: agent_grid_storage_buffers[3].as_entire_binding()
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(&texture_views[2])
                    },
                ]
            })
        );
    }

    let clear_agent_buffers_compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
        label: Some("Clear Agent Buffers Compute Pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
            label: Some("Clear Agent Buffers Compute Pipeline Layout"),
            bind_group_layouts: &[&clear_agent_buffers_bind_group_layout, &agent_list_bind_group_layout],
            push_constant_ranges: &[]
        })),
        module: &shader.clear_agent_buffers,
        entry_point: "main"
    });

    let agent_compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
        label: Some("Agent Compute Pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
            label: Some("Agent Compute Pipeline Layout"),
            bind_group_layouts: &[&global_layout, &compute_bind_group_layout, &agent_list_bind_group_layout, &agent_compute_bind_group_layout],//&agent_list_bind_group_layout, &agent_compute_bind_group_layout],
            push_constant_ranges: &[]
        })),
        module: &shader.compute_agents,
        entry_point: "main"
    });

    let compute_diffuse_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
        label: Some("Compute Diffuse Pipeline"),
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{
            label: Some("Compute Diffuse Pipeline Layout"),
            bind_group_layouts: &[&global_layout, &compute_bind_group_layout],
            push_constant_ranges: &[]
        })),
        module: &shader.compute_diffuse,
        entry_point: "main"
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
        label: None,
        layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline"),
            bind_group_layouts: &[&global_layout, &render_bind_group_layout],
            push_constant_ranges: &[],
        })),
        vertex: wgpu::VertexState {
            module: &shader.vert_frag_texture_sampler,
            entry_point: "vs_main",
            buffers: &[]
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader.vert_frag_texture_sampler,
            entry_point: "fs_main",
            targets: &[Some(
                wgpu::ColorTargetState{
                    format: texture_format,
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

    GPUTasks{
        uniform_buffer,
        global_bind_group,

        clear_agent_buffers_compute_pipeline,
        clear_agent_buffers_bind_groups,

        agent_compute_pipeline,
        agent_list_bind_groups,
        agent_texture_bind_groups,

        compute_diffuse_pipeline,
        diffuse_compute_bind_groups,

        render_pipeline,
        render_bind_groups,

        active_frame: false,
        vertex_buffer,
        vertex_count,
        instance_count,
    }
}
//        bind{ty:wgpu::BufferBindingType::Storage { read_only: false }};
fn build_bind_group(device: &wgpu::Device, name: &str, contents: &[u8], buffer_count: u32, usage: wgpu::BufferUsages,
                    layout_entry_count: u32, visibility:  wgpu::ShaderStages, has_read_only: bool,
                    mut layout_entry_type: wgpu::BindingType)
                    -> (Vec<wgpu::Buffer>, wgpu::BindGroupLayout, Vec<wgpu::BindGroup>)
{

    let mut buffers = Vec::<wgpu::Buffer>::new();
    for i in 0..buffer_count{
        buffers.push(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{} Buffer {}",name, i)),
                contents, usage,
            }),
        );
    }

    let mut layout_entries = Vec::<wgpu::BindGroupLayoutEntry>::new();
    for i in 0..layout_entry_count{
        if has_read_only && i % 2 == 1{
            //ToDo: how to change the read_only property only?
            layout_entry_type = wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(
                    (contents.len() * mem::size_of::<u8>()) as _,
                )
            };
        }
        layout_entries.push(
            wgpu::BindGroupLayoutEntry {
                binding: i, count: None,
                visibility, ty: layout_entry_type
            }
        )
    }

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
        label: Some(&format!("{} Bind Group Layout", name)),
        entries: &*layout_entries
    });

    let mut bind_groups = Vec::<wgpu::BindGroup>::new();
    for i in 0..(has_read_only as u32 + 1)
    {
        let mut bind_group_entries = Vec::<wgpu::BindGroupEntry>::new();
        for j in 0..layout_entry_count {
            bind_group_entries.push(
                wgpu::BindGroupEntry {
                    binding: j,
                    //(double buffer) frame 0: (0, 1), (2, 3) -> frame 1: (1, 0), (3, 2)
                    resource: buffers[((i + (j % 2)) % 2 + (j - (j % 2))) as usize].as_entire_binding()
                },
            )
        }
        bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some(&format!("{} Group {}", name, i)),
            layout: &bind_group_layout,
            entries: &*bind_group_entries
        }));
    }

    return (buffers, bind_group_layout, bind_groups)
}



























