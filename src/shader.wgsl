struct InVertex {
    [[location(0)]] position: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main(in_vertex: InVertex) -> [[builtin(position)]] vec4<f32> {
    return vec4<f32>(in_vertex.position, 1.0);
}

[[stage(fragment)]]
fn fs_main() -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}