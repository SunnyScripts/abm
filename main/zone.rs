use std::mem;
//Created by Ryan Berg 7/5/22
//ToDo: use async functions
//ToDo:!! Running agent compute for every zone!
use nanorand::{Rng, WyRand};
use crate::layer::{Layer};
use bitflags::bitflags;
use wgpu::BufferAddress;

bitflags! {
    struct AgentType: u32 {
        const NONE = 0;
        const TCELL_WANDER = 1 << 0;
        const TCELL_CHASE_CYTOKINE = 1 << 1;
        const DENDRIDIC_PROMOTE_INFLAMATION = 1 << 2;
        const DENDRIDIC_DOWN_REGULATE_INFLAMATION = 1 << 3;
    }
}

bitflags! {
    struct SignalType: u32 {
        const NONE = 0;
        const CYTOKINE = 1 << 0;
        const ANTIBODY = 1 << 1;
    }
}

pub struct Shaders{
    pub agent_compute: wgpu::ShaderModule,
    pub signal_compute: wgpu::ShaderModule,
    pub signal_draw: wgpu::ShaderModule,
}

pub struct Zone {
    diffuse_layer: Layer,
    // agent_layer: Option<Layer>
}



impl Zone{
    pub fn new(diffuse_size: [u32; 2], shader: Shaders, texture_format: wgpu::TextureFormat, device: &wgpu::Device, queue: &wgpu::Queue) -> Self{

        let mut rng = WyRand::new_seed(2203);
        let mut random = || (rng.generate::<f32>() * 100f32) as u32;

        //Signal Parameters and Buffers
        //4 channels, RGBA
        let mut signal_data = vec![0u8; (4 * diffuse_size[0] * diffuse_size[1]) as usize];
        let mut count = 0;
        for signal in signal_data.chunks_mut(4) {
            signal[0] = 255; signal[1] = 69;
            if count >= 4000 && count < 5600 {
                signal[3] = 255;
            }
            else {
                signal[3] = 0;
            }
            count += 1;
        }

        let signal_vertex_buffer_data = vec![0., 0., 1., 0., 1., 1., 0., 0., 1., 0., 1., 1.];

        // let mut max_agent_count = 10000u32;
        let current_agent_count = 53u32;

        //ToDo: vector not needed if not pushing data
        let mut agent_list = vec![0u32; 5 * current_agent_count as usize];

        let mut zone2_agent_grid_occupancy_data = vec![0u32; (5 * diffuse_size[0] * diffuse_size[1]) as usize];

        for i in 0..current_agent_count{
            let x = random(); let y = random();
            let index = ((y * diffuse_size[0] as u32 + x) * 5) as usize;

            zone2_agent_grid_occupancy_data[index] = AgentType::TCELL_WANDER.bits;
            zone2_agent_grid_occupancy_data[(index as u32 + AgentType::TCELL_WANDER.bits) as usize] = 1;  //count

            agent_list[(i * 5) as usize] = AgentType::TCELL_WANDER.bits;
            agent_list[(i * 5) as usize + 1] = 2;  //current zone
            agent_list[(i * 5) as usize + 2] = x;  //x pos
            agent_list[(i * 5) as usize + 3] = y;  //y pos
            agent_list[(i * 5) as usize + 4] = 100;//life remaining
        }

        let mut zone2_signal_grid_occupancy_data = vec![0u32; (3 * diffuse_size[0] * diffuse_size[1]) as usize];

        let mut count = 0;
        for grid_bin_chunk in zone2_signal_grid_occupancy_data.chunks_mut(3) {
            if count >= 4000 && count < 5600 {
                grid_bin_chunk[0] = SignalType::CYTOKINE.bits;
                grid_bin_chunk[1] = 32767;  //cytokine signal strength. max is the max of a signed 16 bit integer 32767. min is -32768
                grid_bin_chunk[2] = 0;      //antibody signal strength
            }
            count+=1;
        }

        let zone_size_buffer_data = [100u32, 100, 0, 0, 100, 100, 0, 0, 100, 100, 0, 0];

        Zone{
            diffuse_layer: Layer::new(count, 6, 1,
                shader.agent_compute, shader.signal_compute, shader.signal_draw,
                signal_vertex_buffer_data, signal_data, zone_size_buffer_data,
                agent_list, zone2_agent_grid_occupancy_data, zone2_signal_grid_occupancy_data,
                texture_format, device, queue)
        }
    }

    pub fn compute_pass(&mut self, queue: &wgpu::Queue, device: &wgpu::Device)
    {
        let layer = &mut self.diffuse_layer;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Command Encoder") });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Agent Compute Pass") });

            compute_pass.set_pipeline(&layer.agent_compute_pipeline);
            compute_pass.set_bind_group(0, &layer.agent_count_bind_groups[layer.active_frame as u32 as usize], &[]);
            compute_pass.set_bind_group(1, &layer.agent_list_bind_groups[layer.active_frame as u32 as usize], &[]);
            compute_pass.set_bind_group(2, &layer.agent_grid_bind_groups[layer.active_frame as u32 as usize], &[]);
            compute_pass.set_bind_group(3, &layer.signal_grid_bind_groups[layer.active_frame as u32 as usize], &[]);
            compute_pass.dispatch_workgroups(53, 1, 1);
        }
        {
            encoder.copy_buffer_to_buffer(&layer.agent_grid_buffers[((layer.active_frame as u32 + 1) % 2) as usize], 0,
                                          &layer.agent_grid_buffers[layer.active_frame as u32 as usize], 0,
                                          (100 * 100 * 5 * mem::size_of::<u32>()) as usize as BufferAddress);
        }
        queue.submit(Some(encoder.finish()));
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Command Encoder") });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Compute Pass") });

            compute_pass.set_pipeline(&layer.compute_diffuse_pipeline);
            compute_pass.set_bind_group(0, &layer.zone_size_bind_group, &[]);
            compute_pass.set_bind_group(1, &layer.agent_grid_bind_groups[layer.active_frame as u32 as usize], &[]);
            compute_pass.set_bind_group(2, &layer.signal_grid_bind_groups[layer.active_frame as u32 as usize], &[]);
            compute_pass.set_bind_group(3, &layer.storage_texture_bind_group, &[]);
            compute_pass.dispatch_workgroups(100, 100, 1);
        }
        layer.toggle_active_frame();
        {
            encoder.copy_texture_to_texture(layer.storage_texture.as_image_copy(), layer.read_texture.as_image_copy(), layer.texture_size);
            //ToDo: add 2 read textures and 2 storage textures
            // encoder.copy_texture_to_texture(layer.storage_texture.as_image_copy(), layer.read_texture.as_image_copy(), layer.texture_size);
            // encoder.copy_texture_to_texture(layer.storage_texture.as_image_copy(), layer.read_texture.as_image_copy(), layer.texture_size);
        }
        // layer.toggle_active_frame();
        queue.submit(Some(encoder.finish()));
    }
    pub fn draw(&mut self, zone: u32, view: &wgpu::TextureView, queue: &wgpu::Queue, device: &wgpu::Device){
        let layer = &mut self.diffuse_layer;
        // queue.write_buffer(&layer.active_zone_buffer, 0, bytemuck::cast_slice(&[zone]));

        let active_texture_bind_group;
        match zone{
            2 => {active_texture_bind_group = &layer.zone2_final_read_texture_bind_group;}
            3 => {active_texture_bind_group = &layer.zone3_final_read_texture_bind_group;}
            _ => {active_texture_bind_group = &layer.zone1_final_read_texture_bind_group;}
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Command Encoder") });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color{r:0.1, b:0.2, g:0.5, a:1.0}),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&layer.render_pipeline);
            render_pass.set_bind_group(0, &layer.zone_size_bind_group, &[]);
            render_pass.set_bind_group(1, &layer.active_zone_bind_group, &[]);
            render_pass.set_bind_group(2, active_texture_bind_group, &[]);
            render_pass.set_bind_group(3, &layer.sampler_bind_group, &[]);
            render_pass.set_vertex_buffer(0, layer.vertex_buffer.slice(..));
            render_pass.draw(0..layer.vertex_count, 0..layer.instance_count);
        }
        queue.submit(Some(encoder.finish()));
    }
}