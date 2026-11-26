// collision.rs
// Simple collision detection for celestial objects

use raylib::prelude::*;

/// Collision system for preventing camera/ship from passing through objects
pub struct CollisionSystem;

impl CollisionSystem {
    /// Check if a position would collide with any celestial object
    /// Returns adjusted position if collision detected
    pub fn check_and_resolve(
        position: Vector3,
        objects: &[(Vector3, f32)], // (position, radius) pairs
        min_distance: f32,
    ) -> Vector3 {
        let mut result = position;

        for (obj_pos, radius) in objects {
            let dx = position.x - obj_pos.x;
            let dy = position.y - obj_pos.y;
            let dz = position.z - obj_pos.z;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();

            let safe_dist = radius + min_distance;

            if dist < safe_dist && dist > 0.001 {
                // Push camera out to safe distance
                let factor = safe_dist / dist;
                result.x = obj_pos.x + dx * factor;
                result.y = obj_pos.y + dy * factor;
                result.z = obj_pos.z + dz * factor;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_camera_inside_a_planet_is_pushed_to_the_safe_distance() {
        let planets = [(Vector3::new(10.0, 0.0, 0.0), 2.0)];
        let pos = CollisionSystem::check_and_resolve(Vector3::new(11.0, 0.0, 0.0), &planets, 0.5);
        assert!(
            (pos - Vector3::new(12.5, 0.0, 0.0)).length() < 1e-5,
            "{pos:?}"
        );
    }

    #[test]
    fn a_camera_far_away_is_left_alone() {
        let planets = [(Vector3::new(10.0, 0.0, 0.0), 2.0)];
        let start = Vector3::new(0.0, 5.0, 0.0);
        assert_eq!(
            CollisionSystem::check_and_resolve(start, &planets, 0.5),
            start
        );
    }
}
