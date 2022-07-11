//Created by Ryan Berg 7/9/22

use imgui::{Condition, TextureId};
use imgui_wgpu::{Texture, TextureConfig};
use wgpu::Device;
use std::string::String;

pub(crate) struct GPUWindow {
    texture_id: TextureId,
    window_size: Option<[f32; 2]>,
    name: String,
    position: [f32; 2]
}


impl GPUWindow{
    pub fn new(position: [f32; 2], size: [f32; 2], name: &str, renderer: &mut imgui_wgpu::Renderer, device: &wgpu::Device) -> Self{

        let texture_id = renderer.textures.insert(new_texture(renderer, device, size));

        GPUWindow{
            window_size: Some(size),
            texture_id,
            name: String::from(name),
            position
        }
    }
    pub fn update(&mut self, ui: &imgui::Ui, renderer: &mut imgui_wgpu::Renderer, device: &Device) -> Option<(TextureId, [f32; 2])> {

        imgui::Window::new(&self.name)
            .position(self.position, Condition::FirstUseEver)
            .size(self.window_size.unwrap(), Condition::FirstUseEver)
            .build(&ui, ||{
                    self.window_size = Some(ui.content_region_avail());
                    imgui::Image::new(self.texture_id, self.window_size.unwrap()).build(&ui);
            });

        if let Some(window_size) = self.window_size {
            if window_size[0] >= 1. && window_size[1] >= 1. {

                renderer.textures.replace(self.texture_id, new_texture(renderer, device, window_size));
                return Some((self.texture_id, window_size));
            }
        }
        return None;
    }
}

fn new_texture(renderer: &imgui_wgpu::Renderer, device: &wgpu::Device, size: [f32; 2]) -> Texture{
    Texture::new(&device, &renderer, TextureConfig{
        size: wgpu::Extent3d{
            width: size[0] as u32,
            height: size[1] as u32,
            ..Default::default()
        },
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        ..Default::default()
    })
}















