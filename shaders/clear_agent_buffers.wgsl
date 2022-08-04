//Created by Ryan Berg 7/19/22

//Types
let PORTAL = 0;
let AGENT_WANDER = 1;
let AGENT_CHASE = 2;

struct Agent {
    state: f32,
    zone: f32,
    x_position: f32,
    y_position: f32,
    seed: f32,
};

struct Types {
    types: array<i32, 3>
}

@group(0) @binding(0) var<storage, read_write> agent_grid_zone_1_dst: array<Types>;
@group(0) @binding(1) var agent_texture_zone_1: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(2) var<storage, read_write> agent_grid_zone_2_dst: array<Types>;
@group(0) @binding(3) var agent_texture_zone_2: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(4) var<storage, read_write> agent_grid_zone_3_dst: array<Types>;
@group(0) @binding(5) var agent_texture_zone_3: texture_storage_2d<rgba8unorm, write>;

@group(1) @binding(0) var<storage, read> agent_list_src: array<Agent>;
@group(1) @binding(1) var<storage, read_write> agent_list_dst: array<Agent>;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>){
    let total = arrayLength(&agent_list_src);
        let index = global_invocation_id.x;
        if (index >= total) {
            return;
        }
        let position = vec2<i32>(i32(agent_list_src[index].x_position), i32(agent_list_src[index].y_position));

        switch (i32(agent_list_src[index].zone))
        {
            default:{
//                agent_grid_zone_1_dst[index].types = array<i32, 3>(-1, -1, -1);
                textureStore(agent_texture_zone_1, position, vec4<f32>(0., 0., 0., 0.));
            }
            case 2:{
                textureStore(agent_texture_zone_2, position, vec4<f32>(0., 0., 0., 0.));
            }
            case 3:{
                textureStore(agent_texture_zone_3, position, vec4<f32>(0., 0., 0., 0.));
            }
        }

}











