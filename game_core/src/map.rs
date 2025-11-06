use glam::Vec2;

/// Axis-aligned bounding box
#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        let half = size * 0.5;
        Self {
            min: center - half,
            max: center + half,
        }
    }

    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    /// Check if circle intersects AABB
    pub fn intersects_circle(&self, center: Vec2, radius: f32) -> bool {
        let closest = Vec2::new(
            center.x.clamp(self.min.x, self.max.x),
            center.y.clamp(self.min.y, self.max.y),
        );
        (center - closest).length_squared() <= radius * radius
    }
}

/// Map definition
#[derive(Debug, Clone)]
pub struct Map {
    pub blocks: Vec<Aabb>,
    pub spawns: Vec<Vec2>,
    pub hill_center: Vec2,
    pub pickup_pads: Vec<(Vec2, crate::components::PickupKind)>,
}

impl Map {
    /// Create a simple test map
    pub fn test_map() -> Self {
        let mut blocks = Vec::new();
        let mut spawns = Vec::new();
        let hill_center = Vec2::ZERO;

        // Create a simple arena with walls
        let arena_size = 20.0;
        let wall_thickness = 1.0;

        // Outer walls
        blocks.push(Aabb::new(
            Vec2::new(-arena_size, -arena_size),
            Vec2::new(arena_size, -arena_size + wall_thickness),
        ));
        blocks.push(Aabb::new(
            Vec2::new(-arena_size, arena_size - wall_thickness),
            Vec2::new(arena_size, arena_size),
        ));
        blocks.push(Aabb::new(
            Vec2::new(-arena_size, -arena_size),
            Vec2::new(-arena_size + wall_thickness, arena_size),
        ));
        blocks.push(Aabb::new(
            Vec2::new(arena_size - wall_thickness, -arena_size),
            Vec2::new(arena_size, arena_size),
        ));

        // Some interior obstacles
        blocks.push(Aabb::from_center_size(
            Vec2::new(-8.0, 0.0),
            Vec2::new(2.0, 2.0),
        ));
        blocks.push(Aabb::from_center_size(
            Vec2::new(8.0, 0.0),
            Vec2::new(2.0, 2.0),
        ));
        blocks.push(Aabb::from_center_size(
            Vec2::new(0.0, -8.0),
            Vec2::new(2.0, 2.0),
        ));
        blocks.push(Aabb::from_center_size(
            Vec2::new(0.0, 8.0),
            Vec2::new(2.0, 2.0),
        ));

        // Spawn points (corners and sides)
        spawns.push(Vec2::new(-15.0, -15.0));
        spawns.push(Vec2::new(15.0, -15.0));
        spawns.push(Vec2::new(-15.0, 15.0));
        spawns.push(Vec2::new(15.0, 15.0));
        spawns.push(Vec2::new(0.0, -15.0));
        spawns.push(Vec2::new(0.0, 15.0));
        spawns.push(Vec2::new(-15.0, 0.0));
        spawns.push(Vec2::new(15.0, 0.0));

        // Pickup pads (8-12 pads)
        let mut pickup_pads = Vec::new();
        for i in 0..10 {
            let angle = (i as f32) * std::f32::consts::TAU / 10.0;
            let dist = 12.0;
            let pos = Vec2::new(angle.cos() * dist, angle.sin() * dist);
            let kind = match i % 3 {
                0 => crate::components::PickupKind::Health,
                1 => crate::components::PickupKind::BoltUpgrade,
                _ => crate::components::PickupKind::ShieldModule,
            };
            pickup_pads.push((pos, kind));
        }

        Self {
            blocks,
            spawns,
            hill_center,
            pickup_pads,
        }
    }
}

