//ToDo: use z to process each zone texture. need additional texture buffers

struct AgentGridBin {
  occupant_bit_flags: i32,
  tcell_wander_count: i32,
  tcell_chase_cytokine_count: i32,
  dendritic_promote_inflamation_count: i32,
  dendritic_down_regulate_inflamation_count: i32,
};

struct SignalGridBin {
  occupant_bit_flags: f32,
  cytokine_signal_strength: f32,
  antibody_signal_strength: f32,
};

struct wrapped_f32 { //to meet uniform array stride requirement
  @size(16) elem: vec2<f32>
}

struct  UniformData{
    accumulator_delta_time: vec2<f32>,
    zone1_dim: vec2<f32>,
    window1_dim: vec2<f32>,
    zone2_dim: vec2<f32>,
    window2_dim: vec2<f32>,
    zone3_dim: vec2<f32>,
//    window3_dim: vec2<f32>,
    neighbors: array<vec4<f32>, 2>,//array is the 7th item in struct to satisfy uniform offset requirement
}

@group(0) @binding(0) var<uniform> u: UniformData;

@group(1) @binding(0) var<storage, read> zone1_agent_grid_occupants_src: array<AgentGridBin>;
@group(1) @binding(2) var<storage, read> zone2_agent_grid_occupants_src: array<AgentGridBin>;
@group(1) @binding(4) var<storage, read> zone3_agent_grid_occupants_src: array<AgentGridBin>;

@group(2) @binding(0) var<storage, read> zone1_signal_grid_occupants_src: array<SignalGridBin>;
@group(2) @binding(1) var<storage, read_write> zone1_signal_grid_occupants_dst: array<SignalGridBin>;
@group(2) @binding(2) var<storage, read> zone2_signal_grid_occupants_src: array<SignalGridBin>;
@group(2) @binding(3) var<storage, read_write> zone2_signal_grid_occupants_dst: array<SignalGridBin>;
@group(2) @binding(4) var<storage, read> zone3_signal_grid_occupants_src: array<SignalGridBin>;
@group(2) @binding(5) var<storage, read_write> zone3_signal_grid_occupants_dst: array<SignalGridBin>;

@group(3) @binding(0) var storage_texture_zone_1: texture_storage_2d<rgba8unorm, write>;
@group(3) @binding(1) var storage_texture_zone_2: texture_storage_2d<rgba8unorm, write>;
@group(3) @binding(2) var storage_texture_zone_3: texture_storage_2d<rgba8unorm, write>;

let diffuse_constant = 1.;
let evaporation_rate = .99;
let MIN = 0.;
let MAX = 1.;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>){

    var agent_intensity = 0.;
    var signal_intensity = 0.;

    let index = global_invocation_id.y * u32(u.zone1_dim.x) + global_invocation_id.x;

//    if (global_invocation_id.z == u32(0)){
    if (zone1_agent_grid_occupants_src[index].occupant_bit_flags > 0) {agent_intensity = 1.;}

    signal_intensity = zone1_signal_grid_occupants_src[index].cytokine_signal_strength;// * 32767.;

    var sum = 0.;
    for(var row_offset: i32 = -1; row_offset < 2; row_offset++) {
        for(var column_offset: i32 = -1; column_offset < 2; column_offset++) {
            if(row_offset == 0 && column_offset == 0) { continue; }
            var neighbor = vec2<i32>(global_invocation_id.xy);

            neighbor.y = neighbor.y + row_offset;
            if(neighbor.y < 0) {neighbor.y = i32(u.zone1_dim.x) - 1;}
            else if(neighbor.y > i32(u.zone1_dim.x) - 1) {neighbor.y = 0;}

            neighbor.x = neighbor.x + column_offset;
            if(neighbor.x < 0) {neighbor.x = i32(u.zone1_dim.x) - 1;}
            else if(neighbor.x > i32(u.zone1_dim.x) - 1) {neighbor.x = 0;}

            let neighbor_index = u32(neighbor.y * i32(u.zone1_dim.y) + neighbor.x);


            if(row_offset == 0 || column_offset == 0)
            { sum = sum + 4. *  zone1_signal_grid_occupants_src[neighbor_index].cytokine_signal_strength; }
            else
            { sum = sum + zone1_signal_grid_occupants_src[neighbor_index].cytokine_signal_strength; }
        }
    }

    sum = sum - 20. * signal_intensity;
    let delta = sum / 20.;
    signal_intensity = signal_intensity + delta * diffuse_constant;
//    signal_intensity = signal_intensity / 32767.;
//    d = d * evaporation_rate;
    signal_intensity = clamp(signal_intensity, MIN, MAX);

    zone1_signal_grid_occupants_dst[index].cytokine_signal_strength = signal_intensity;

    if(i32(global_invocation_id.y) < 99){
        textureStore(storage_texture_zone_1, vec2<i32>(global_invocation_id.xy),
                vec4<f32>(.22, .7 * agent_intensity, .85 * signal_intensity, 1.));
    }
    else{
        textureStore(storage_texture_zone_1, vec2<i32>(global_invocation_id.xy),
                        vec4<f32>(0., 0., 0., 1.));
    }
}
