// vertex.rs
// Vértice de un modelo .obj

use raylib::math::Vector3;

#[derive(Clone, Debug)]
pub struct Vertex {
    /// Posición en espacio de modelo
    pub position: Vector3,
    /// Normal en espacio de modelo
    pub normal: Vector3,
}
