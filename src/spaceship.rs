// spaceship.rs
// Spaceship that follows the camera using loaded OBJ model

use crate::obj::Obj;
use crate::shader_system::ShaderType;
use crate::vertex::Vertex;

/// Spaceship configuration and transform calculations
pub struct Spaceship {
    pub vertices: Vec<Vertex>,
    pub shader_type: ShaderType,
    pub scale: f32,
}

impl Spaceship {
    /// Load spaceship from OBJ file
    pub fn load(path: &str, scale: f32) -> Result<Self, tobj::LoadError> {
        let obj = Obj::load(path)?;
        let vertices = obj.get_vertex_array();

        Ok(Spaceship {
            vertices,
            shader_type: ShaderType::Spaceship,
            scale,
        })
    }

    /// Get the vertex array for rendering
    pub fn get_vertices(&self) -> &[Vertex] {
        &self.vertices
    }
}
