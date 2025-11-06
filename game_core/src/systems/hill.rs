use hecs::World;

use crate::components::*;
use crate::params::Params;
use crate::resources::*;

/// Update hill scoring
pub fn hill_score_tick(
    world: &mut World,
    time: &Time,
    map: &GameMap,
    score: &mut Score,
    config: &Config,
) {
    if !config.objective_on {
        return;
    }

    // Find hill zone
    let hill_zone = map.map.hill_center;
    let hill_r = Params::HILL_RADIUS;

    // Find all players in hill
    let mut players_in_hill = Vec::new();

    for (_entity, (player, transform)) in world.query::<(&Player, &Transform2D)>().iter() {
        let dist = (transform.pos - hill_zone).length();
        if dist <= hill_r {
            players_in_hill.push(player.id);
        }
    }

    // Only score if exactly one player in hill
    if players_in_hill.len() == 1 {
        let player_id = players_in_hill[0];
        // Accumulate fractional points
        let fractional = score.hill_points_fractional.entry(player_id).or_insert(0.0);
        *fractional += Params::HILL_POINTS_PER_SEC as f32 * time.dt;

        // Convert whole points
        let whole_points = *fractional as u16;
        if whole_points > 0 {
            *fractional -= whole_points as f32;
            let points = score.hill_points.entry(player_id).or_insert(0);
            *points = points.saturating_add(whole_points);
        }
    }
}
