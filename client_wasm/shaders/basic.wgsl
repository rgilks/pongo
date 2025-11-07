// Basic forward pass shader for ISO game
// Simple lambert lighting + ambient

// Camera uniform buffer (binding 0)
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye: vec4<f32>,
    _padding: array<f32, 12>,
}
@group(0) @binding(0) var<uniform> camera: CameraUniform;

// Light data (binding 1)
struct Light {
    pos: vec3<f32>,
    radius: f32,
    color: vec3<f32>,
    intensity: f32,
}
@group(1) @binding(1) var<storage, read> lights: array<Light>;
@group(1) @binding(2) var<uniform> light_count: u32;

// Vertex input
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

// Instance data (for instanced rendering)
struct InstanceData {
    @location(2) transform: vec4<f32>, // x, y, scale, rotation
    @location(3) tint: vec4<f32>,     // rgba
}

// Vertex output
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(4) color: vec4<f32>, // Changed from 2 to 4 to avoid conflict with instance data
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceData,
) -> VertexOutput {
    // Extract instance transform
    let pos_2d = instance.transform.xy;
    let scale = instance.transform.z;
    let rotation = instance.transform.w;
    
    // Transform to world space (2D game, Y is up in 3D)
    let cos_r = cos(rotation);
    let sin_r = sin(rotation);
    let rot_matrix = mat3x3<f32>(
        vec3<f32>(cos_r, 0.0, -sin_r),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(sin_r, 0.0, cos_r),
    );
    
    let world_pos = rot_matrix * (model.position * scale) + vec3<f32>(pos_2d.x, 0.0, pos_2d.y);
    let world_normal = rot_matrix * model.normal;
    
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_position = world_pos;
    out.normal = normalize(world_normal);
    out.color = instance.tint;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Ambient lighting
    let ambient = 0.2;
    
    // Accumulate point lights
    var light_contrib = vec3<f32>(0.0);
    let view_dir = normalize(camera.eye.xyz - in.world_position);
    
    for (var i: u32 = 0u; i < light_count; i++) {
        let light = lights[i];
        let to_light = light.pos - in.world_position;
        let dist = length(to_light);
        
        // Distance attenuation
        if (dist < light.radius) {
            let dir = normalize(to_light);
            let ndotl = max(dot(in.normal, dir), 0.0);
            let attenuation = 1.0 - (dist / light.radius);
            light_contrib += light.color * light.intensity * ndotl * attenuation;
        }
    }
    
    // Simple lambert + ambient
    let final_color = in.color.rgb * (ambient + light_contrib);
    return vec4<f32>(final_color, in.color.a);
}

