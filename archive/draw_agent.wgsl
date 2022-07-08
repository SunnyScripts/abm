
//    @location(0) particle_pos: vec2<f32>,
//    @location(1) particle_vel: vec2<f32>,
//    @location(2) position: vec2<f32>,

@group(1) @binding(0) var<storage, read> particlesSrc : array<f32>;
@group(1) @binding(1) var<storage, read_write> particlesDst : array<f32>;

@vertex
fn vs_main() -> @builtin(position) vec4<f32> {
//    let angle = -atan2(particle_vel.x, particle_vel.y);
//    let pos = vec2<f32>(
//        position.x * cos(angle) - position.y * sin(angle),
//        position.x * sin(angle) + position.y * cos(angle)
//    );
//    return vec4<f32>(pos + particle_pos, 0.0, 1.0);
return vec4<f32>(0., 0., 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1., 1., 1., 1.);
}