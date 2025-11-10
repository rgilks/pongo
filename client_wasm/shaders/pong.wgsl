// Simple 2D shader for Pong

// Camera uniform (256-byte aligned)
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

// Vertex input (mesh vertices)
struct VertexInput {
    @location(0) position: vec3<f32>,
};

// Instance input (per-object data)
struct InstanceInput {
    @location(1) transform: vec4<f32>,  // x, y, scale_x, scale_y
    @location(2) tint: vec4<f32>,       // rgba color
};

// Vertex output / Fragment input
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Apply instance transform (scale and translate)
    let scaled_pos = vec3<f32>(
        vertex.position.x * instance.transform.z,  // scale_x
        vertex.position.y * instance.transform.w,  // scale_y
        vertex.position.z
    );
    
    let world_pos = vec4<f32>(
        scaled_pos.x + instance.transform.x,  // translate x
        scaled_pos.y + instance.transform.y,  // translate y
        scaled_pos.z,
        1.0
    );
    
    out.clip_position = camera.view_proj * world_pos;
    out.color = instance.tint;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}

