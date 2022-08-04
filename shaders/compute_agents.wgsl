//Created by Ryan Berg 7/7/22

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

struct  UniformData{
    time: f32,
    zone_offset: i32,
    zone1_dim: vec2<f32>,
    window1_dim: vec2<f32>,
    zone2_dim: vec2<f32>,
    window2_dim: vec2<f32>,
    zone3_dim: vec2<f32>,
//    window3_dim: vec2<f32>,
    neighbors: array<vec4<f32>, 9>,//array is the 7th item in struct with a stride of 16Bs to satisfy uniform array alignment and offset requirement
    colors: array<vec4<f32>, 4>,
}

@group(0) @binding(0) var<uniform> u: UniformData;

@group(1) @binding(0) var read_diffuse_texture_zone_1: texture_2d<f32>;
@group(1) @binding(1) var write_diffuse_texture_zone_1: texture_storage_2d<rgba8unorm, write>;
@group(1) @binding(2) var read_diffuse_texture_zone_2: texture_2d<f32>;
@group(1) @binding(3) var write_diffuse_texture_zone_2: texture_storage_2d<rgba8unorm, write>;
@group(1) @binding(4) var read_diffuse_texture_zone_3: texture_2d<f32>;
@group(1) @binding(5) var write_diffuse_texture_zone_3: texture_storage_2d<rgba8unorm, write>;

@group(2) @binding(0) var<storage, read> agent_list_src: array<Agent>;
@group(2) @binding(1) var<storage, read_write> agent_list_dst: array<Agent>;
//
@group(3) @binding(0) var agent_texture_zone_1: texture_storage_2d<rgba8unorm, write>;
@group(3) @binding(1) var<storage, read> agent_grid_zone_1_src: array<Types>;
@group(3) @binding(2) var<storage, read_write> agent_grid_zone_1_dst: array<Types>;
@group(3) @binding(3) var agent_texture_zone_2: texture_storage_2d<rgba8unorm, write>;
@group(3) @binding(4) var<storage, read> agent_grid_zone_2_src: array<Types>;
@group(3) @binding(5) var<storage, read_write> agent_grid_zone_2_dst: array<Types>;
@group(3) @binding(6) var agent_texture_zone_3: texture_storage_2d<rgba8unorm, write>;
@group(3) @binding(7) var<storage, read> agent_grid_zone_3_src: array<Types>;
@group(3) @binding(8) var<storage, read_write> agent_grid_zone_3_dst: array<Types>;

//uniform RNG from https://www.shadertoy.com/view/4t2SDh
//normalized to [0..1]
fn nrand( n: vec2<f32> ) -> f32
{
	return fract(sin(dot(n.xy, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}
fn n1rand( n: vec2<f32> ) -> f32
{
	return nrand( n + 0.07 * fract(u.time) );
}

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {

    let total = arrayLength(&agent_list_src);
    let index = global_invocation_id.x;
    if (index >= total) {
        return;
    }

    var position = vec2<f32>(agent_list_src[index].x_position, agent_list_src[index].y_position);
    var zone = agent_list_src[index].zone;
//    let cell_type = agent_list_src[index].old_state;
//
//    var cell_color = vec3<f32>(.45, .85, 0.);
//    switch (i32(cell_type)){
//        default :{}
//        case 1 :{
//            cell_color = vec3<f32>(0., 1., 0.);
//        }
//        case 2 :{
//            cell_color = vec3<f32>(0., .33, 0.4);
//        }
//    }
//
    var zone_dimensions = u.zone1_dim;
    if(i32(zone) == 1) { zone_dimensions = u.zone2_dim; }
    else if(i32(zone) == 2) { zone_dimensions = u.zone3_dim; }

    var grid_index = i32(position.y * zone_dimensions.y + position.x);


    if(agent_list_src[index].state == f32(PORTAL)) {

       switch (i32(zone)){
           default :{
                textureStore(agent_texture_zone_1, vec2<i32>(position), vec4<f32>(.25, .2, 0.05, 1.));
           }
           case 2:{
               textureStore(agent_texture_zone_2, vec2<i32>(position), vec4<f32>(.25, .2, 0.05, 1.));
           }
           case 3:{
               textureStore(agent_texture_zone_3, vec2<i32>(position), vec4<f32>(.25, .2, 0.05, 1.));
           }
       }

       return;
    }

//region Agent Chase Logic
//
//    var strongest_neighbor_signal_direction = vec2<f32>(0., 0.);
//    var strongest_neighbor_signal = textureLoad(read_diffuse_texture_zone_1, vec2<i32>(position), 0).a;
//
//    for(var row_offset: i32 = -1; row_offset < 2; row_offset++) {
//        for(var column_offset: i32 = -1; column_offset < 2; column_offset++) {
//            if(row_offset == 0 && column_offset == 0) { continue; }
//            var neighbor = vec2<i32>(position);
//
//            neighbor.y = neighbor.y + row_offset;
//            if(neighbor.y < 0) {neighbor.y = i32(zone_dimensions.x) - 1;}
//            else if(neighbor.y > i32(zone_dimensions.x) - 1) {neighbor.y = 0;}
//
//            neighbor.x = neighbor.x + column_offset;
//            if(neighbor.x < 0) {neighbor.x = i32(zone_dimensions.x) - 1;}
//            else if(neighbor.x > i32(zone_dimensions.x) - 1) {neighbor.x = 0;}
//
////            let neighbor_index = neighbor.y * i32(zone_dimensions.y) + neighbor.x;
//
//            let signal = textureLoad(read_diffuse_texture_zone_1, neighbor, 0).a;
//            if(signal > strongest_neighbor_signal + .005){
//                strongest_neighbor_signal = signal;
//                strongest_neighbor_signal_direction = vec2<f32>(f32(column_offset), f32(row_offset));
//            }
//        }
//    }
//endregion

    var move_direction = vec2<f32>(0., 0.);
//    var current_state = AGENT_WANDER;
//    if(strongest_neighbor_signal > .05) {
//        move_direction = move_direction + strongest_neighbor_signal_direction;
//        current_state = AGENT_CHASE;
//    }
//    else {
        let l_uv = position / zone_dimensions;
        let u_rand = n1rand( (l_uv + agent_list_src[index].seed) * .5 );

        move_direction = u.neighbors[u32(floor(u_rand * 9. - .01))].xy;
//    }

    position = position + move_direction;

    if(position.y > 99.) {position.y = 0.;}
    else if(position.y < 0.) {position.y = 99.;}
    if(position.x > 99.) {position.x = 0.;}
    else if(position.x < 0.) {position.x = 99.;}

    grid_index = i32(position.y * zone_dimensions.y + position.x);
    var portal_index = -1;

    switch (i32(zone)){
        default :{
           portal_index = agent_grid_zone_1_src[grid_index].types[0];
        }
        case 2:{
           portal_index = agent_grid_zone_2_src[grid_index].types[0];
        }
        case 3:{
           portal_index = agent_grid_zone_3_src[grid_index].types[0];
        }
    }

    if(portal_index > -1) {
        let portal_sibling = agent_list_src[ i32(agent_list_src[portal_index].seed) ];
        position = vec2<f32>(portal_sibling.x_position, portal_sibling.y_position);
        zone = portal_sibling.zone;
        agent_list_dst[index].zone = zone;
    }

    agent_list_dst[index].x_position = position.x;
    agent_list_dst[index].y_position = position.y;

    switch (i32(zone)){
        default :{
            textureStore(write_diffuse_texture_zone_1, vec2<i32>(position), vec4<f32>(0., 0., .85, 1.));
        }
        case 2:{
            textureStore(write_diffuse_texture_zone_2, vec2<i32>(position), vec4<f32>(0., 0., .85, 1.));
        }
        case 3:{
            textureStore(write_diffuse_texture_zone_3, vec2<i32>(position), vec4<f32>(0., 0., .85, 1.));
        }
    }
}

















