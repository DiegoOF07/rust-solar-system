// solar_system.rs
// Solar system scene with multiple celestial objects

use crate::shader_system::ShaderType;
use raylib::prelude::*;
use std::f32::consts::PI;

/// Celestial object types
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CelestialType {
    Star,
    Planet,
}

/// Represents a celestial object
#[derive(Clone, Debug)]
pub struct CelestialObject {
    pub object_type: CelestialType,
    pub shader_type: ShaderType,
    pub position: Vector3,
    pub scale: f32,
    pub rotation: Vector3,
    pub rotation_speed: Vector3,
    pub orbit_radius: f32,
    pub orbit_speed: f32,
    pub orbit_angle: f32,
    pub parent_index: Option<usize>,
}

impl CelestialObject {
    /// Create a new star
    pub fn star(scale: f32) -> Self {
        CelestialObject {
            object_type: CelestialType::Star,
            shader_type: ShaderType::Star,
            position: Vector3::zero(),
            scale,
            rotation: Vector3::zero(),
            rotation_speed: Vector3::new(0.0, 0.02, 0.0),
            orbit_radius: 0.0,
            orbit_speed: 0.0,
            orbit_angle: 0.0,
            parent_index: None,
        }
    }

    /// Create a new planet
    pub fn planet(
        parent_idx: usize,
        orbit_radius: f32,
        orbit_speed: f32,
        scale: f32,
        shader: ShaderType,
    ) -> Self {
        CelestialObject {
            object_type: CelestialType::Planet,
            shader_type: shader,
            position: Vector3::new(orbit_radius, 0.0, 0.0),
            scale,
            rotation: Vector3::zero(),
            rotation_speed: Vector3::new(0.0, 0.03, 0.0),
            orbit_radius,
            orbit_speed,
            orbit_angle: (rand::random::<f32>() * PI * 2.0), // Ángulo inicial aleatorio para desincronizar
            parent_index: Some(parent_idx),
        }
    }
}

/// Solar system container
pub struct SolarSystem {
    pub objects: Vec<CelestialObject>,
}

impl SolarSystem {
    /// Create empty solar system
    pub fn new() -> Self {
        SolarSystem {
            objects: Vec::new(),
        }
    }

    /// Add an object and return its index
    pub fn add(&mut self, object: CelestialObject) -> usize {
        self.objects.push(object);
        self.objects.len() - 1
    }

    /// Update all objects
    pub fn update(&mut self, delta_time: f32) {
        let len = self.objects.len();
        // Update rotations
        for obj in &mut self.objects {
            // Si es planeta, rota sobre su eje Y
            if obj.object_type == CelestialType::Planet {
                obj.rotation.y += 1.0 * delta_time; // Puedes ajustar la velocidad aquí
            }
            obj.rotation.x += obj.rotation_speed.x * delta_time;
            obj.rotation.y += obj.rotation_speed.y * delta_time;
            obj.rotation.z += obj.rotation_speed.z * delta_time;
        }
        // Update orbital positions (hierarchical)
        for i in 0..len {
            if let Some(parent_idx) = self.objects[i].parent_index {
                let parent_pos = self.objects[parent_idx].position;
                let obj = &mut self.objects[i];

                // Aplicar ley de Kepler: velocidad más lenta mientras más lejos
                // v = orbit_speed / sqrt(orbit_radius)
                let adjusted_speed = obj.orbit_speed / obj.orbit_radius.sqrt();

                obj.orbit_angle += adjusted_speed * 1.0 * delta_time;
                obj.position.x = parent_pos.x + obj.orbit_radius * obj.orbit_angle.cos();
                obj.position.z = parent_pos.z + obj.orbit_radius * obj.orbit_angle.sin();
                obj.position.y = parent_pos.y;
            }
        }
    }

    /// Create a basic solar system preset
    pub fn create_basic_system() -> Self {
        let mut system = SolarSystem::new();

        // Central star (Sun)
        let sun_idx = system.add(CelestialObject::star(4.0));

        // Inner rocky planet (Mercury-like)
        system.add(CelestialObject::planet(
            sun_idx,
            8.0,
            0.08,
            0.4,
            ShaderType::Rocky,
        ));

        // Second planet (Venus-like - lava world)
        system.add(CelestialObject::planet(
            sun_idx,
            12.0,
            0.06,
            0.65,
            ShaderType::Lava,
        ));

        // Planeta tipo Tierra
        system.add(CelestialObject::planet(
            sun_idx,
            17.0,
            0.05,
            1.0,
            ShaderType::CloudPlanet,
        ));

        // Ocean planet - Mundo acuático
        system.add(CelestialObject::planet(
            sun_idx,
            20.5,
            0.045,
            0.9,
            ShaderType::Ocean,
        ));

        // Gas giant (Jupiter-like)
        system.add(CelestialObject::planet(
            sun_idx,
            27.0,
            0.03,
            1.5,
            ShaderType::GasGiant,
        ));

        // Desert planet
        system.add(CelestialObject::planet(
            sun_idx,
            31.0,
            0.025,
            0.7,
            ShaderType::Desert,
        ));

        // Striped gas giant
        system.add(CelestialObject::planet(
            sun_idx,
            37.0,
            0.02,
            1.3,
            ShaderType::Striped,
        ));

        // Outer ice world
        system.add(CelestialObject::planet(
            sun_idx,
            45.0,
            0.015,
            0.8,
            ShaderType::IceWorld,
        ));

        system
    }
}
