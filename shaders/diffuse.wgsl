//ToDo: use z to process each zone texture. need additional texture buffers

struct AgentGridBin {
  occupant_bit_flags: u32,
  tcell_wander_count: u32,
  tcell_chase_cytokine_count: u32,
  dendritic_promote_inflamation_count: u32,
  dendritic_down_regulate_inflamation_count: u32,
};

struct SignalGridBin {
  occupant_bit_flags: u32,
  cytokine_signal_strength: u32,
  antibody_signal_strength: u32,
};

struct ZoneSize {
  dimensions: vec2<u32>,
  resolution: vec2<u32>,
};

@group(0) @binding(0) var<uniform> zone_size: ZoneSize;

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

let width: i32 = 100;
let diffuse_constant = 1.;
let evaporation_rate = .99;
let MIN = 0.;
let MAX = 1.;

@compute
@workgroup_size(64)
fn main(@builtin(global_invocation_id) global_invocation_id: vec3<u32>){

    var agent_intensity = 0.;
    var signal_intensity = 0.;

    let index = global_invocation_id.y * u32(width) + global_invocation_id.x;

    if (global_invocation_id.z == u32(0)){
        if (zone1_agent_grid_occupants_src[index].occupant_bit_flags != u32(0)) {agent_intensity = 1.;}
        if (zone1_signal_grid_occupants_src[index].occupant_bit_flags != u32(0)) {signal_intensity = 1.;}
        textureStore(storage_texture_zone_1, vec2<i32>(global_invocation_id.xy),
        vec4<f32>(.85 * signal_intensity, .7 * agent_intensity, .2, 1.));
    }
//    else if(global_invocation_id.z == u32(1)){
//        agent_color.a = f32(zone2_agent_grid_occupants_src[index].occupant_bit_flags / zone2_agent_grid_occupants_src[index].occupant_bit_flags);
//        signal_color.a = f32(zone2_signal_grid_occupants_src[index].occupant_bit_flags / zone2_signal_grid_occupants_src[index].occupant_bit_flags);
//        textureStore(storage_texture_zone_2, vec2<i32>(global_invocation_id.xy), agent_color-signal_color);
//    }
//    else{
//        agent_color.a = f32(zone3_agent_grid_occupants_src[index].occupant_bit_flags / zone3_agent_grid_occupants_src[index].occupant_bit_flags);
//        signal_color.a = f32(zone3_signal_grid_occupants_src[index].occupant_bit_flags / zone3_signal_grid_occupants_src[index].occupant_bit_flags);
//        textureStore(storage_texture_zone_3, vec2<i32>(global_invocation_id.xy), agent_color-signal_color);
//    }
//    textureStore(storage_texture_zone_3, vec2<i32>(global_invocation_id.xy), agent_color-signal_color);
return;
}

//        switch (global_invocation_id.z){
//            case 0:{
//                agent_color.a = f32(zone1_agent_grid_occupants_src[index].occupant_bit_flags / zone1_agent_grid_occupants_src[index].occupant_bit_flags);
//                signal_color.a = f32(zone1_signal_grid_occupants_src[index].occupant_bit_flags / zone1_signal_grid_occupants_src[index].occupant_bit_flags);
//                textureStore(storage_texture_zone_1, vec2<i32>(global_invocation_id.xy), agent_color+signal_color);
//            }
//            case 1:{
//                agent_color.a = f32(zone2_agent_grid_occupants_src[index].occupant_bit_flags / zone2_agent_grid_occupants_src[index].occupant_bit_flags);
//                signal_color.a = f32(zone2_signal_grid_occupants_src[index].occupant_bit_flags / zone2_signal_grid_occupants_src[index].occupant_bit_flags);
//                textureStore(storage_texture_zone_2, vec2<i32>(global_invocation_id.xy), agent_color+signal_color);
//            }
//            default:{
//                agent_color.a = f32(zone3_agent_grid_occupants_src[index].occupant_bit_flags / zone3_agent_grid_occupants_src[index].occupant_bit_flags);
//                signal_color.a = f32(zone3_signal_grid_occupants_src[index].occupant_bit_flags / zone3_signal_grid_occupants_src[index].occupant_bit_flags);
//                textureStore(storage_texture_zone_3, vec2<i32>(global_invocation_id.xy), agent_color+signal_color);
//            }
//        }

//    let local_coord = vec2<i32>(global_invocation_id.xy);
//
//    let texel = textureLoad(t_diffuse, vec2<i32>(local_coord), i32(0));
//
//    var sum = 0.;
//
//    for(var row_offset: i32 = -1; row_offset < 2; row_offset++) {
//        for(var column_offset: i32 = -1; column_offset < 2; column_offset++) {
//            if(row_offset == 0 && column_offset == 0) { continue; }
//            var neighbor = local_coord;
//
//            neighbor.y = neighbor.y + row_offset;
//            if(neighbor.y < 0) {neighbor.y = width - 1;}
//            else if(neighbor.y > width - 1) {neighbor.y = 0;}
//
//            neighbor.x = neighbor.x + column_offset;
//            if(neighbor.x < 0) {neighbor.x = width - 1;}
//            else if(neighbor.x > width - 1) {neighbor.x = 0;}
//
//
//            if(row_offset == 0 || column_offset == 0)
//            { sum = sum + 4. *  textureLoad(t_diffuse, vec2<i32>(neighbor), i32(0)).a; }
//            else
//            { sum = sum + textureLoad(t_diffuse, vec2<i32>(neighbor), i32(0)).a; }
//        }
//    }
//
//    sum = sum - 20. * texel.a;
//    let delta = sum / 20.;
//    var d = texel.a + delta * diffuse_constant;
////    d = d * evaporation_rate;
//    d = clamp(d, MIN, MAX);
//
//
//
//    textureStore(storage_texture, vec2<i32>(global_invocation_id.xy), vec4<f32>(0., 1., .5, d));

//    particlesDst[index] = d;
