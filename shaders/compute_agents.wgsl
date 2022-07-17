//Created by Ryan Berg 7/7/22
struct Agent {
  state: f32,
  zone: f32,
  x_coord: f32,
  y_coord: f32,
  seed: f32,
};

struct  UniformData{
    time: vec2<f32>,
    zone1_dim: vec2<f32>,
    window1_dim: vec2<f32>,
    zone2_dim: vec2<f32>,
    window2_dim: vec2<f32>,
    zone3_dim: vec2<f32>,
//    window3_dim: vec2<f32>,
    neighbors: array<vec4<f32>, 9>,//array is the 7th item in struct with a stride of 16Bs to satisfy uniform array alignment and offset requirement
}

struct AgentGridBin {
  occupant_bit_flags: i32,
};

struct SignalGridBin {
  occupant_bit_flags: f32,
  cytokine_signal_strength: f32,
  antibody_signal_strength: f32,
};

let agent_wander_color = vec4<f32>(.3882, .7725, .8588, 1.);
let agent_chase_color = vec4<f32>(.3882, .8588, .56078, 1.);

@group(0) @binding(0) var<uniform> u: UniformData;
//@group(0) @binding(1) var noise_texture: texture_2d<f32>;

@group(1) @binding(0) var<storage, read> agent_list_src: array<Agent>;
@group(1) @binding(1) var<storage, read_write> agent_list_dst: array<Agent>;

@group(2) @binding(0) var<storage, read> zone1_agent_grid_occupants_src: array<AgentGridBin>;
@group(2) @binding(1) var<storage, read_write> zone1_agent_grid_occupants_dst: array<AgentGridBin>;
@group(2) @binding(2) var<storage, read> zone2_agent_grid_occupants_src: array<AgentGridBin>;
@group(2) @binding(3) var<storage, read_write> zone2_agent_grid_occupants_dst: array<AgentGridBin>;
@group(2) @binding(4) var<storage, read> zone3_agent_grid_occupants_src: array<AgentGridBin>;
@group(2) @binding(5) var<storage, read_write> zone3_agent_grid_occupants_dst: array<AgentGridBin>;

@group(3) @binding(0) var<storage, read_write> zone1_signal_grid_occupants_src: array<SignalGridBin>;
@group(3) @binding(1) var<storage, read_write> zone1_signal_grid_occupants_dst: array<SignalGridBin>;
@group(3) @binding(2) var<storage, read_write> zone2_signal_grid_occupants_src: array<SignalGridBin>;
@group(3) @binding(3) var<storage, read_write> zone2_signal_grid_occupants_dst: array<SignalGridBin>;
@group(3) @binding(4) var<storage, read_write> zone3_signal_grid_occupants_src: array<SignalGridBin>;
@group(3) @binding(5) var<storage, read_write> zone3_signal_grid_occupants_dst: array<SignalGridBin>;

//uniform RNG from https://www.shadertoy.com/view/4t2SDh
//normalized to [0..1]
fn nrand( n: vec2<f32> ) -> f32
{
	return fract(sin(dot(n.xy, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}
fn n1rand( n: vec2<f32> ) -> f32
{
	return nrand( n + 0.07 * fract(u.time.x) );
}

fn read_signal_zone(zone_index: u32, grid_index: u32) -> f32
{
    switch (i32(zone_index)){
        default :{
            return zone1_signal_grid_occupants_src[grid_index].cytokine_signal_strength;
        }
        case 2:{
            return zone2_signal_grid_occupants_src[grid_index].cytokine_signal_strength;
        }
        case 3:{
            return zone3_signal_grid_occupants_src[grid_index].cytokine_signal_strength;
        }
    }
}
fn write_agent_grid_zone(zone_index: u32, grid_index: u32, state: i32)
{
    switch (i32(zone_index)){
        default :{
            zone1_agent_grid_occupants_dst[grid_index].occupant_bit_flags = state;
        }
        case 2:{
            zone2_agent_grid_occupants_dst[grid_index].occupant_bit_flags = state;
        }
        case 3:{
            zone3_agent_grid_occupants_dst[grid_index].occupant_bit_flags = state;
        }
    }
}

fn write_signal_grid_zone(zone_index: u32, grid_index: u32, signal: f32)
{
    switch (i32(zone_index)){
        default :{
            zone1_signal_grid_occupants_src[grid_index].cytokine_signal_strength = signal;
        }
        case 2:{
            zone2_signal_grid_occupants_src[grid_index].cytokine_signal_strength = signal;
        }
        case 3:{
            zone3_signal_grid_occupants_src[grid_index].cytokine_signal_strength = signal;
        }
    }
}

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {

  let total = arrayLength(&agent_list_src);
  let index = global_invocation_id.x;
  if (index >= total) {
    return;
  }

  var position = vec2<f32>(agent_list_src[index].x_coord, agent_list_src[index].y_coord);
  let grid_index = u32(position.y * u.zone1_dim.x + position.x);
  let zone = u32(agent_list_src[index].zone);

  if(agent_list_src[index].state == 1.) {
    write_signal_grid_zone(zone, grid_index, 1.);
    write_agent_grid_zone(zone, grid_index, 1);
    return;
  }

  var zone_dimensions = u.zone1_dim;
  if(zone == u32(1)) { zone_dimensions = u.zone2_dim; }
  else if(zone == u32(2)) { zone_dimensions = u.zone3_dim; }


    var strongest_neighbor_signal_direction = vec2<f32>(0., 0.);
    var strongest_neighbor_signal = read_signal_zone(zone, grid_index);
    for(var row_offset: i32 = -1; row_offset < 2; row_offset++) {
        for(var column_offset: i32 = -1; column_offset < 2; column_offset++) {
            if(row_offset == 0 && column_offset == 0) { continue; }
            var neighbor = vec2<i32>(position);

            neighbor.y = neighbor.y + row_offset;
            if(neighbor.y < 0) {neighbor.y = i32(zone_dimensions.x) - 1;}
            else if(neighbor.y > i32(zone_dimensions.x) - 1) {neighbor.y = 0;}

            neighbor.x = neighbor.x + column_offset;
            if(neighbor.x < 0) {neighbor.x = i32(zone_dimensions.x) - 1;}
            else if(neighbor.x > i32(zone_dimensions.x) - 1) {neighbor.x = 0;}
            
            let neighbor_index = u32(neighbor.y * i32(zone_dimensions.y) + neighbor.x);
            
            let signal = read_signal_zone(zone, neighbor_index);
            if(signal > strongest_neighbor_signal + .001){
                strongest_neighbor_signal = signal;
                strongest_neighbor_signal_direction = vec2<f32>(f32(column_offset), f32(row_offset));
            }
        }
    }

    var move_direction = vec2<f32>(0., 0.);
    var current_state = 2;
    if(strongest_neighbor_signal > .033)
    {
        move_direction = move_direction + strongest_neighbor_signal_direction;
        current_state = 3;
    }
    else{
        let l_uv = position / zone_dimensions;
        let u_rand = n1rand( (l_uv + agent_list_src[index].seed + (f32(zone) / 3.)) * .3334 );

        move_direction = u.neighbors[u32(floor(u_rand * 9. - .01))].xy;
    }

    position = position + move_direction;

    if(position.y > 99.) {position.y = 0.;}
    else if(position.y < 0.) {position.y = 99.;}
    if(position.x > 99.) {position.x = 0.;}
    else if(position.x < 0.) {position.x = 99.;}

    let next_grid_index = u32(position.y * zone_dimensions.y + position.x);
    agent_list_dst[index].x_coord = position.x; agent_list_dst[index].y_coord = position.y;

    write_agent_grid_zone(zone, next_grid_index, current_state);
}

















