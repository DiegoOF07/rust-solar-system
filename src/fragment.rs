// fragment.rs
// Datos de un píxel visible que reciben los shaders

use raylib::math::Vector3;

pub struct Fragment {
    /// Gris base ya iluminado (Lambert)
    pub color: Vector3,
    /// Posición en espacio de modelo, para el ruido procedural
    pub world_position: Vector3,
}
