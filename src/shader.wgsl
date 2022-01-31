struct InVertex {
    [[location(0)]] position: vec2<f32>;
};

struct InInstance {
    [[location(1)]] translation: vec2<f32>;
    [[location(2)]] rotation: f32;
    [[location(3)]] scale: vec2<f32>;
};

struct Uniforms {
    aspect_ratio: f32;
};

[[group(0), binding(0)]]
var<uniform> uniforms: Uniforms;

[[stage(vertex)]]
fn vs_main(in_vertex: InVertex, in_instance: InInstance) -> [[builtin(position)]] vec4<f32> {
    var scaled = in_vertex.position * in_instance.scale;
    var rotated = vec2<f32>(
        scaled.x * cos(in_instance.rotation) - scaled.y * sin(in_instance.rotation),
        scaled.x * sin(in_instance.rotation) + scaled.y * cos(in_instance.rotation),
    );
    var translated = rotated + in_instance.translation;
    var view_transformed = vec2<f32>(translated.x * uniforms.aspect_ratio, translated.y);
    var final = view_transformed;

    return vec4<f32>(final, 0.0, 1.0);
}

[[stage(fragment)]]
fn fs_main() -> [[location(0)]] vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}