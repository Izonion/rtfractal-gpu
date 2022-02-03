struct InVertex {
    [[location(0)]] position: vec2<f32>;
};

struct OutVertex {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] tex_coord: vec2<f32>;
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
fn vs_main(in_vertex: InVertex, in_instance: InInstance) -> OutVertex {
    var out_vertex: OutVertex;

    out_vertex.tex_coord = (in_vertex.position + vec2<f32>(1.0, 1.0)) / 2.0;

    var scaled = in_vertex.position * in_instance.scale;
    var rotated = vec2<f32>(
        scaled.x * cos(in_instance.rotation) - scaled.y * sin(in_instance.rotation),
        scaled.x * sin(in_instance.rotation) + scaled.y * cos(in_instance.rotation),
    );
    var translated = rotated + in_instance.translation;
    var view_transformed = vec2<f32>(translated.x * uniforms.aspect_ratio, translated.y);
    out_vertex.position = vec4<f32>(view_transformed, 0.0, 1.0);

    return out_vertex;
}

[[group(0), binding(1)]]
var t_diffuse: texture_2d<f32>;
[[group(0), binding(2)]]
var s_diffuse: sampler;

struct OutFrag {
    [[location(0)]] normal: vec4<f32>;
    [[location(1)]] other: vec4<f32>;
};

[[stage(fragment)]]
fn fs_main(out_vertex: OutVertex) -> OutFrag {
    var out_frag: OutFrag;
    out_frag.normal = textureSample(t_diffuse, s_diffuse, out_vertex.tex_coord);
    out_frag.other = textureSample(t_diffuse, s_diffuse, out_vertex.tex_coord);
    return out_frag;
}