use game_core::{
    create_ball, create_paddle, step, Ball, Config, Events, GameMap, GameRng, NetQueue, Paddle,
    RespawnState, Score, Time,
};
use hecs::World;

pub struct LocalGame {
    pub world: World,
    pub time: Time,
    pub map: GameMap,
    pub config: Config,
    pub score: Score,
    pub events: Events,
    pub net_queue: NetQueue,
    pub rng: GameRng,
    pub respawn_state: RespawnState,
}

impl LocalGame {
    pub fn new(seed: u64) -> Self {
        let map = GameMap::new();
        let config = Config::new();
        let mut world = World::new();
        let mut rng = GameRng::new(seed);

        // Create paddles
        let left_paddle_y = map.paddle_spawn(0).y;
        let right_paddle_y = map.paddle_spawn(1).y;
        create_paddle(&mut world, 0, left_paddle_y);
        create_paddle(&mut world, 1, right_paddle_y);

        // Create ball
        let mut ball = Ball::new(glam::f32::Vec2::ZERO, glam::f32::Vec2::ZERO);
        ball.reset(config.ball_speed_initial, &mut rng);
        create_ball(&mut world, ball.pos, ball.vel);

        Self {
            world,
            time: Time::new(0.016, 0.0),
            map,
            config,
            score: Score::new(),
            events: Events::new(),
            net_queue: NetQueue::new(),
            rng,
            respawn_state: RespawnState::new(),
        }
    }

    pub fn step(
        &mut self,
        my_paddle_y: f32,
    ) -> (
        Option<u8>,
        Option<(glam::Vec2, glam::Vec2)>,
        f32,
        f32,
        u8,
        u8,
    ) {
        // AI: Control right paddle (player_id=1)
        let ai_dir = calculate_ai_input(&self.world, &self.config);
        
        const SIM_FIXED_DT: f32 = 1.0 / 60.0; // Assume standard step for AI movement

        // Update AI paddle position locally
        let mut ai_y = 12.0;
        // Find current AI paddle y
        for (_e, paddle) in self.world.query::<&Paddle>().iter() {
            if paddle.player_id == 1 {
                ai_y = paddle.y;
                break;
            }
        }
        
        let mut new_ai_y = ai_y + (ai_dir as f32) * self.config.paddle_speed * SIM_FIXED_DT;
        // Clamp
        let half_height = self.config.paddle_height / 2.0;
        new_ai_y = new_ai_y.clamp(half_height, self.config.arena_height - half_height);

        self.net_queue.push_input(0, my_paddle_y);
        self.net_queue.push_input(1, new_ai_y);


        self.time = Time::new(SIM_FIXED_DT, self.time.now + SIM_FIXED_DT);

        step(
            &mut self.world,
            &mut self.time,
            &self.map,
            &self.config,
            &mut self.score,
            &mut self.events,
            &mut self.net_queue,
            &mut self.rng,
            &mut self.respawn_state,
        );

        let winner = self.score.has_winner(self.config.win_score);

        // Extract data needed for visual updates
        let ball_data = self
            .world
            .query::<&Ball>()
            .iter()
            .next()
            .map(|(_e, ball)| (ball.pos, ball.vel));

        let mut paddle_left_y = 12.0;
        let mut paddle_right_y = 12.0;
        for (_e, paddle) in self.world.query::<&Paddle>().iter() {
            if paddle.player_id == 0 {
                paddle_left_y = paddle.y;
            } else if paddle.player_id == 1 {
                paddle_right_y = paddle.y;
            }
        }

        (
            winner,
            ball_data,
            paddle_left_y,
            paddle_right_y,
            self.score.left,
            self.score.right,
        )
    }
}

/// Calculate AI input for opponent paddle
///
/// Strategy:
/// 1. Simple heuristic: if ball is moving towards us, predict intersection y.
/// 2. If intersection is significantly different from current y, move there.
/// 3. If ball moving away, return to center to cover maximum area.
fn calculate_ai_input(world: &World, config: &Config) -> i8 {
    let ball_data = world
        .query::<&Ball>()
        .iter()
        .next()
        .map(|(_e, ball)| (ball.pos, ball.vel));
    let paddle_data = world
        .query::<&Paddle>()
        .iter()
        .find(|(_e, p)| p.player_id == 1)
        .map(|(_e, p)| p.y);

    if let (Some((ball_pos, ball_vel)), Some(paddle_y)) = (ball_data, paddle_data) {
        if ball_vel.x > 0.0 {
            let paddle_x = config.paddle_x(1);
            let time_to_reach = (paddle_x - ball_pos.x) / ball_vel.x.max(0.1);
            let predicted_y = ball_pos.y + ball_vel.y * time_to_reach;

            let target_y = predicted_y + (ball_vel.y * 0.3);
            let diff = target_y - paddle_y;
            let deadzone = 0.3;

            if diff > deadzone {
                1
            } else if diff < -deadzone {
                -1
            } else {
                0
            }
        } else {
            let center_y = 12.0;
            let diff = center_y - paddle_y;
            if diff.abs() > 0.5 {
                if diff > 0.0 {
                    1
                } else {
                    -1
                }
            } else {
                0
            }
        }
    } else {
        0
    }
}
