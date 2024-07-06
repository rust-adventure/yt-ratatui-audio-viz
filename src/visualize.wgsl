@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    // one triangle to cover the whole window
    let x = f32(i32(in_vertex_index & 1u)) * 4.0 - 1.0;
    let y = f32(max(in_vertex_index, 1u)) * 4.0 - 5.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}
@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>( pos.xy/pos.w, 0.0, 1.0);
}