//Created by Ryan Berg 7/7/22
struct Agent {
  state: i32,
  zone: i32,
  x_coord: i32,
  y_coord: i32,
  seed: i32,
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
  occupant_bit_flags : i32,
  tcell_wander_count : i32,
  tcell_chase_cytokine_count : i32,
  dendritic_promote_inflamation_count : i32,
  dendritic_down_regulate_inflamation_count : i32,
};

struct SignalGridBin {
  occupant_bit_flags : i32,
  cytokine_signal_strength : i32,
  antibody_signal_strength : i32,
};

//@group(0) @binding(0) var<storage, read> agent_count_src : u32;//in agent list
//@group(0) @binding(1) var<storage, read_write> agent_count_dst : u32;

@group(1) @binding(0) var<storage, read> agent_list_src : array<Agent>;
@group(1) @binding(1) var<storage, read_write> agent_list_dst : array<Agent>;

@group(2) @binding(0) var<storage, read> zone1_agent_grid_occupants_src : array<AgentGridBin>;
@group(2) @binding(1) var<storage, read_write> zone1_agent_grid_occupants_dst : array<AgentGridBin>;
@group(2) @binding(2) var<storage, read> zone2_agent_grid_occupants_src : array<AgentGridBin>;
@group(2) @binding(3) var<storage, read_write> zone2_agent_grid_occupants_dst : array<AgentGridBin>;
@group(2) @binding(4) var<storage, read> zone3_agent_grid_occupants_src : array<AgentGridBin>;
@group(2) @binding(5) var<storage, read_write> zone3_agent_grid_occupants_dst : array<AgentGridBin>;

@group(3) @binding(0) var<storage, read> zone1_signal_grid_occupants_src : array<SignalGridBin>;
@group(3) @binding(2) var<storage, read> zone2_signal_grid_occupants_src : array<SignalGridBin>;
@group(3) @binding(4) var<storage, read> zone3_signal_grid_occupants_src : array<SignalGridBin>;

//@group(0) @binding(0) var<uniform> u: array<UniformData>;
@group(0) @binding(0) var<uniform> u: UniformData;

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

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {

  let total = arrayLength(&agent_list_src);
  let index = global_invocation_id.x;
  if (index >= total) {
    return;
  }


    var position = vec2<i32>(i32(agent_list_src[index].x_coord), i32(agent_list_src[index].y_coord));
    let grid_index = position.y * i32(u.zone1_dim.y) + position.x;
    let agent_state = agent_list_src[index].state;

    let l_uv = vec2<f32>(position) / u.zone1_dim;
//    let normalized_grid_index = l_uv.y * u.zone1_dim.y + l_uv.x;

    let u_rand = n1rand( l_uv );
//    agent_list_dst[index].seed = agent_list_src[index].seed + u32(((u_rand + 2.) * .5) * 2203.);

    let move_direction = u.neighbors[u32(floor(u_rand * 9. - .01))];

    position = vec2<i32>(vec2<f32>(position) + move_direction.xy);

    if(position.y > 99) {position.y = 0;}
    else if(position.y < 0) {position.y = 99;}
    if(position.x > 99) {position.x = 0;}
    else if(position.x < 0) {position.x = 99;}

    let next_grid_index = position.y * i32(u.zone1_dim.y) + position.x;
    agent_list_dst[index].x_coord = position.x; agent_list_dst[index].y_coord = position.y;


    zone1_agent_grid_occupants_dst[next_grid_index].occupant_bit_flags = zone1_agent_grid_occupants_src[next_grid_index].occupant_bit_flags + 1;
    zone1_agent_grid_occupants_dst[next_grid_index].tcell_wander_count = zone1_agent_grid_occupants_src[next_grid_index].tcell_wander_count + 1;

    zone1_agent_grid_occupants_dst[grid_index].tcell_wander_count = zone1_agent_grid_occupants_dst[grid_index].tcell_wander_count  - 1;
    zone1_agent_grid_occupants_dst[grid_index].occupant_bit_flags = zone1_agent_grid_occupants_dst[grid_index].occupant_bit_flags  - 1;



//
//  let plus_one = agent_list_src[0].state + u32(1);
//
//  agent_list_dst[0].state = plus_one;

//  textureStore(storage_texture, vec2<i32>(global_invocation_id.xy), vec4<f32>(0., 1., .5, d));
}
