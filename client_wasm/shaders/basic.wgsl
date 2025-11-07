// Basic forward pass shader for ISO game
// Simple lambert lighting + ambient

// Camera uniform buffer (binding 0)
// Note: Uniform buffers require 16-byte alignment, so padding must use vec4<f32> (16 bytes) instead of f32 (4 bytes)
struct CameraUniform {
    view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    eye: vec4<f32>,
    _padding: array<vec4<f32>, 3>, // 3 * vec4 = 12 floats = 48 bytes, properly aligned
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
    // Simple mapping: game (x, y) -> 3D (x, 0, z)
    // Apply player rotation (yaw) around Y axis
    let cos_r = cos(rotation);
    let sin_r = sin(rotation);
    let rot_matrix = mat3x3<f32>(
        vec3<f32>(cos_r, 0.0, -sin_r),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(sin_r, 0.0, cos_r),
    );
    
    // Map 2D position to 3D: (x, y) -> (x, 0, z)
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
    // Enhanced ambient lighting with color
    let ambient_color = vec3<f32>(0.1, 0.13, 0.2); // Deeper cool blue ambient
    let ambient_strength = 0.6;
    
    // Directional light from above (sun/moon) - stronger and more angled
    let light_dir = normalize(vec3<f32>(0.2, 1.0, 0.2));
    let light_color = vec3<f32>(1.0, 0.99, 0.95); // Very warm, bright sunlight
    let light_strength = 1.0;
    let ndotl = max(dot(in.normal, light_dir), 0.0);
    // Add specular-like highlight for more depth
    let specular = pow(max(dot(normalize(light_dir + normalize(camera.eye.xyz - in.world_position)), in.normal), 0.0), 32.0) * 0.3;
    let directional = light_color * light_strength * (ndotl + specular);
    
    // Accumulate point lights (for glowing effects)
    var light_contrib = vec3<f32>(0.0);
    let view_dir = normalize(camera.eye.xyz - in.world_position);
    
    for (var i: u32 = 0u; i < light_count; i++) {
        let light = lights[i];
        let to_light = light.pos - in.world_position;
        let dist = length(to_light);
        
        // Distance attenuation with smoother falloff
        if (dist < light.radius) {
            let dir = normalize(to_light);
            let ndotl_point = max(dot(in.normal, dir), 0.0);
            let attenuation = pow(1.0 - (dist / light.radius), 2.5); // Stronger falloff
            light_contrib += light.color * light.intensity * (ndotl_point + 0.3) * attenuation; // Add base glow
        }
    }
    
    // Combine all lighting with better contrast
    let base_lighting = ambient_color * ambient_strength + directional;
    let lit_color = in.color.rgb * base_lighting;
    
    // Add rim lighting for depth and glow effect
    let rim = pow(1.0 - max(dot(view_dir, in.normal), 0.0), 3.0);
    let rim_color = in.color.rgb * rim * 0.4; // Rim for depth
    
    // Add point light glow (unlit contribution for emissive feel)
    let glow = light_contrib * 0.5; // Unlit glow contribution for brighter objects
    
    // Final color with better contrast
    let final_color = lit_color + rim_color + glow + light_contrib;
    
    // Slight color boost for saturation (make colors pop more)
    let boosted = mix(final_color, in.color.rgb * 1.15, 0.15);
    return vec4<f32>(boosted, in.color.a);
}

