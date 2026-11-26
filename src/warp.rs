// warp.rs
// Salto animado hacia un cuerpo celeste y seguimiento mientras orbita

use raylib::prelude::*;

/// Duración del salto en segundos
const WARP_DURATION: f32 = 0.8;

enum WarpState {
    Idle,
    Warping {
        start_eye: Vector3,
        start_target: Vector3,
        body: usize,
        progress: f32,
    },
    /// Llegó al cuerpo y lo acompaña en su órbita
    Following {
        body: usize,
        last_position: Vector3,
    },
}

pub struct WarpSystem {
    state: WarpState,
    current: Option<usize>,
}

impl WarpSystem {
    pub fn new() -> Self {
        WarpSystem {
            state: WarpState::Idle,
            current: None,
        }
    }

    pub fn warp_to(&mut self, body: usize, eye: Vector3, target: Vector3) {
        self.state = WarpState::Warping {
            start_eye: eye,
            start_target: target,
            body,
            progress: 0.0,
        };
        self.current = Some(body);
    }

    /// Índice del siguiente cuerpo, en ciclo
    pub fn next_body(&self, count: usize) -> usize {
        self.current.map_or(1 % count, |i| (i + 1) % count)
    }

    pub fn stop_following(&mut self) {
        self.state = WarpState::Idle;
    }

    pub fn is_warping(&self) -> bool {
        matches!(self.state, WarpState::Warping { .. })
    }

    /// Nueva posición de la cámara (ojo, objetivo), o None si no hay salto ni seguimiento.
    /// `bodies` son las posiciones y radios actuales.
    pub fn update(
        &mut self,
        delta_time: f32,
        bodies: &[(Vector3, f32)],
        eye: Vector3,
        target: Vector3,
    ) -> Option<(Vector3, Vector3)> {
        match &mut self.state {
            WarpState::Idle => None,
            WarpState::Warping {
                start_eye,
                start_target,
                body,
                progress,
            } => {
                let (position, radius) = *bodies.get(*body)?;
                let view = radius * 4.0 + 2.0;
                let end_eye = position + Vector3::new(view, view * 0.5, view);
                *progress = (*progress + delta_time / WARP_DURATION).min(1.0);
                let t = *progress;
                let ease = if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                };
                let result = (
                    start_eye.lerp(end_eye, ease),
                    start_target.lerp(position, ease),
                );
                if t >= 1.0 {
                    self.state = WarpState::Following {
                        body: *body,
                        last_position: position,
                    };
                }
                Some(result)
            }
            WarpState::Following {
                body,
                last_position,
            } => {
                let (position, _) = *bodies.get(*body)?;
                let delta = position - *last_position;
                *last_position = position;
                Some((eye + delta, target + delta))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bodies(planet: Vector3) -> Vec<(Vector3, f32)> {
        vec![(Vector3::zero(), 4.0), (planet, 1.0)]
    }

    fn run(
        warp: &mut WarpSystem,
        steps: usize,
        dt: f32,
        planet: impl Fn(usize) -> Vector3,
    ) -> (Vector3, Vector3) {
        let (mut eye, mut target) = (Vector3::new(0.0, 25.0, 40.0), Vector3::zero());
        for i in 0..steps {
            if let Some((e, t)) = warp.update(dt, &bodies(planet(i)), eye, target) {
                eye = e;
                target = t;
            }
        }
        (eye, target)
    }

    #[test]
    fn the_jump_lands_on_where_the_planet_is_now() {
        let mut warp = WarpSystem::new();
        warp.warp_to(1, Vector3::new(0.0, 25.0, 40.0), Vector3::zero());
        // El planeta se mueve durante todo el salto
        let (eye, target) = run(&mut warp, 60, 1.0 / 60.0, |i| {
            Vector3::new(10.0 + i as f32 * 0.1, 0.0, 0.0)
        });
        let planet_now = Vector3::new(10.0 + 59.0 * 0.1, 0.0, 0.0);
        assert!(
            (target - planet_now).length() < 1e-3,
            "mira a {target:?}, el planeta está en {planet_now:?}"
        );
        let distance = (eye - planet_now).length();
        assert!(distance > 1.5 && distance < 12.0, "distancia {distance}");
    }

    #[test]
    fn after_the_jump_the_camera_follows_the_planet() {
        let mut warp = WarpSystem::new();
        warp.warp_to(1, Vector3::new(0.0, 25.0, 40.0), Vector3::zero());
        let (eye_a, _) = run(&mut warp, 60, 1.0 / 60.0, |_| Vector3::new(10.0, 0.0, 0.0));
        // Ya llegó; ahora el planeta avanza 3 unidades en z
        let (eye_b, target_b) = {
            let mut eye = eye_a;
            let mut target = Vector3::new(10.0, 0.0, 0.0);
            for z in [1.0, 2.0, 3.0] {
                if let Some((e, t)) =
                    warp.update(1.0 / 60.0, &bodies(Vector3::new(10.0, 0.0, z)), eye, target)
                {
                    eye = e;
                    target = t;
                }
            }
            (eye, target)
        };
        assert!((target_b - Vector3::new(10.0, 0.0, 3.0)).length() < 1e-4);
        assert!((eye_b - (eye_a + Vector3::new(0.0, 0.0, 3.0))).length() < 1e-4);
    }

    #[test]
    fn stopping_leaves_the_camera_where_it_is() {
        let mut warp = WarpSystem::new();
        warp.warp_to(1, Vector3::new(0.0, 25.0, 40.0), Vector3::zero());
        run(&mut warp, 60, 1.0 / 60.0, |_| Vector3::new(10.0, 0.0, 0.0));
        warp.stop_following();
        let moved = warp.update(
            1.0 / 60.0,
            &bodies(Vector3::new(10.0, 0.0, 5.0)),
            Vector3::zero(),
            Vector3::zero(),
        );
        assert!(moved.is_none());
    }

    #[test]
    fn next_cycles_through_the_bodies() {
        let mut warp = WarpSystem::new();
        assert_eq!(warp.next_body(3), 1);
        warp.warp_to(2, Vector3::zero(), Vector3::zero());
        assert_eq!(warp.next_body(3), 0);
    }
}
