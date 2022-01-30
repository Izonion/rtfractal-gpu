struct InVertex {
    [[location(0)]] position: vec2<f32>;
};

struct Uniforms {
    aspect_ratio: f32;
};

[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

[[stage(vertex)]]
fn vs_main(in_vertex: InVertex) -> [[builtin(position)]] vec4<f32> {
    return vec4<f32>(in_vertex.position.x * uniforms.aspect_ratio, in_vertex.position.y, 0.0, 1.0);
}

[[stage(fragment)]]
fn fs_main() -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}