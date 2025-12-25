use wgpu::*;
#[allow(unused_imports)]
use crate::state::GameState;
use super::Renderer;
use super::resources::InstanceData;

pub fn draw_frame(
    renderer: &mut Renderer,
    game_state: &GameState,
    local_paddle_y: f32,
    is_local_game: bool,
) -> Result<(), String> {
    let output = renderer.surface.get_current_texture()
        .map_err(|e| format!("Failed to get current texture: {:?}", e))?;
    let view = output.texture.create_view(&TextureViewDescriptor::default());
    let mut encoder = renderer.device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    update_buffers(renderer, game_state, local_paddle_y, is_local_game);

    if renderer.enable_trails {
        render_with_trails(renderer, &mut encoder, &view);
    } else {
        render_basic(renderer, &mut encoder, &view);
    }

    renderer.queue.submit(std::iter::once(encoder.finish()));
    output.present();

    Ok(())
}

fn update_buffers(renderer: &mut Renderer, game_state: &GameState, local_paddle_y: f32, is_local_game: bool) {
    let paddle_left_x = 1.5;
    let paddle_right_x = 30.5;
    let paddle_width = 0.8;
    let paddle_height = 4.0;
    let ball_radius = 0.5;

    let my_player_id = game_state.get_player_id();

    let left_paddle_y = if !is_local_game && my_player_id == Some(0) {
        local_paddle_y
    } else {
        game_state.get_paddle_left_y()
    };

    let right_paddle_y = if !is_local_game && my_player_id == Some(1) {
        local_paddle_y
    } else {
        game_state.get_paddle_right_y()
    };

    let left_instance = InstanceData {
        transform: [paddle_left_x, left_paddle_y, paddle_width, paddle_height],
        tint: [0.0, 1.0, 0.0, 1.0],
    };
    let right_instance = InstanceData {
        transform: [paddle_right_x, right_paddle_y, paddle_width, paddle_height],
        tint: [0.0, 1.0, 0.0, 1.0],
    };
    let ball_instance = InstanceData {
        transform: [
            game_state.get_ball_x(),
            game_state.get_ball_y(),
            ball_radius * 2.0,
            ball_radius * 2.0,
        ],
        tint: [1.0, 1.0, 1.0, 1.0],
    };

    let current = (left_instance, right_instance, ball_instance);
    let needs_update = renderer.last_instance_data.map(|last| {
            last.0.transform != current.0.transform
            || last.1.transform != current.1.transform 
            || last.2.transform != current.2.transform
    }).unwrap_or(true);

    if needs_update {
            renderer.queue.write_buffer(&renderer.buffers.left_paddle, 0, bytemuck::cast_slice(&[left_instance]));
            renderer.queue.write_buffer(&renderer.buffers.right_paddle, 0, bytemuck::cast_slice(&[right_instance]));
            renderer.queue.write_buffer(&renderer.buffers.ball, 0, bytemuck::cast_slice(&[ball_instance]));
            renderer.last_instance_data = Some(current);
    }
}

fn render_with_trails(renderer: &mut Renderer, encoder: &mut CommandEncoder, view: &TextureView) {
    // Ping-pong technique:
    // We have two textures, A and B.
    // Frame N: Read from A, Write to B.
    // Frame N+1: Read from B, Write to A.
    // This allows us to feed the previous frame's trail back into the solution to create fading trails.
    let (write_view, read_group) = if renderer.trail_use_a {
        (&renderer.textures.view_a, &renderer.trail_bind_group_b)
    } else {
            (&renderer.textures.view_b, &renderer.trail_bind_group_a)
    };

    // 1. Write current game objects (paddles, ball) to the trail texture.
    // This captures the "fresh" positions for the trail.
    {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Trail Write"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: write_view,
                resolve_target: None,
                ops: Operations { load: LoadOp::Clear(Color::TRANSPARENT), store: StoreOp::Store },
            })],
            depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None,
        });
        draw_objects(renderer, &mut pass);
    }

    // 2. Fade previous trail into the same texture.
    // We draw the *previous* frame's texture onto the current one with high transparency.
    // This effectively "updates" the trail state by slightly dimming old pixels and keeping new ones.
    {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Trail Fade"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: write_view,
                resolve_target: None,
                ops: Operations { load: LoadOp::Load, store: StoreOp::Store },
            })],
            depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None,
        });
        pass.set_pipeline(&renderer.trail_pipeline);
        pass.set_bind_group(0, read_group, &[]);
        pass.set_vertex_buffer(0, renderer.buffers.trail_vertex.slice(..));
        pass.draw(0..4, 0..1);
    }

    // 3. Main Pass
    {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Main Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations { load: LoadOp::Clear(Color::BLACK), store: StoreOp::Store },
            })],
            depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None,
        });
        
        // Draw the faded trail texture as a background layer.
        // This creates the "motion blur" effect by accumulating past frames.
        pass.set_pipeline(&renderer.trail_pipeline);
        pass.set_bind_group(0, read_group, &[]);
        pass.set_vertex_buffer(0, renderer.buffers.trail_vertex.slice(..));
        pass.draw(0..4, 0..1);

        // Draw the actual game objects on top of the trails.
        draw_objects(renderer, &mut pass);
    }

    // Swap the ping-pong flag so the texture we just wrote to becomes the read source next frame.
    renderer.trail_use_a = !renderer.trail_use_a;
}

fn render_basic(renderer: &Renderer, encoder: &mut CommandEncoder, view: &TextureView) {
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("Main Pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: Operations { load: LoadOp::Clear(Color::BLACK), store: StoreOp::Store },
        })],
        depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None,
    });
    draw_objects(renderer, &mut pass);
}

fn draw_objects<'a>(renderer: &'a Renderer, pass: &mut RenderPass<'a>) {
    pass.set_pipeline(&renderer.main_pipeline);
    pass.set_bind_group(0, &renderer.camera_bind_group, &[]);
    
    // Rects (Paddles)
    pass.set_vertex_buffer(0, renderer.meshes.0.vertex_buffer.slice(..));
    pass.set_index_buffer(renderer.meshes.0.index_buffer.slice(..), IndexFormat::Uint16);
    
    pass.set_vertex_buffer(1, renderer.buffers.left_paddle.slice(..));
    pass.draw_indexed(0..renderer.meshes.0.index_count, 0, 0..1);

    pass.set_vertex_buffer(1, renderer.buffers.right_paddle.slice(..));
    pass.draw_indexed(0..renderer.meshes.0.index_count, 0, 0..1);

    // Circle (Ball)
    pass.set_vertex_buffer(0, renderer.meshes.1.vertex_buffer.slice(..));
    pass.set_index_buffer(renderer.meshes.1.index_buffer.slice(..), IndexFormat::Uint16);
    pass.set_vertex_buffer(1, renderer.buffers.ball.slice(..));
    pass.draw_indexed(0..renderer.meshes.1.index_count, 0, 0..1);
}
