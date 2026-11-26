// framebuffer.rs
// Buffers de color, profundidad y muestras visibles, y su presentación en la ventana

use crate::shader_system::ShaderType;
use raylib::prelude::*;
use rayon::prelude::*;

/// Lo que el shader necesita del fragmento visible en un píxel.
/// Solo es válido donde el píxel tiene profundidad; por eso no hace falta limpiarlo.
#[derive(Clone, Copy)]
pub struct Sample {
    pub model_pos: Vector3,
    pub intensity: f32,
    pub shader: ShaderType,
}

const EMPTY_SAMPLE: Sample = Sample {
    model_pos: Vector3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    },
    intensity: 0.0,
    shader: ShaderType::Rocky,
};

pub struct Framebuffer {
    pub width: i32,
    pub height: i32,
    color: Vec<[u8; 4]>,
    depth: Vec<f32>,
    samples: Vec<Sample>,
    background: [u8; 4],
    texture: Option<Texture2D>,
}

impl Framebuffer {
    pub fn new(width: i32, height: i32) -> Self {
        let len = (width * height) as usize;
        Framebuffer {
            width,
            height,
            color: vec![[0, 0, 0, 255]; len],
            depth: vec![f32::INFINITY; len],
            samples: vec![EMPTY_SAMPLE; len],
            background: [0, 0, 0, 255],
            texture: None,
        }
    }

    pub fn set_background_color(&mut self, color: Color) {
        self.background = [color.r, color.g, color.b, 255];
    }

    pub fn clear(&mut self) {
        let background = self.background;
        self.color.par_iter_mut().for_each(|c| *c = background);
        self.depth.par_iter_mut().for_each(|d| *d = f32::INFINITY);
    }

    /// Pinta un píxel del fondo (estrellas, órbitas); los cuerpos se dibujan encima
    pub fn point_no_depth(&mut self, x: i32, y: i32, color: Vector3) {
        if x >= 0 && x < self.width && y >= 0 && y < self.height {
            let index = (y * self.width + x) as usize;
            if self.depth[index] == f32::INFINITY {
                self.color[index] = to_rgba(color);
            }
        }
    }

    pub(crate) fn depth_and_samples_mut(&mut self) -> (&mut [f32], &mut [Sample]) {
        (&mut self.depth, &mut self.samples)
    }

    pub(crate) fn color_depth_samples_mut(&mut self) -> (&mut [[u8; 4]], &[f32], &[Sample]) {
        (&mut self.color, &self.depth, &self.samples)
    }

    /// Sube el cuadro a una textura que se reutiliza y la dibuja
    pub fn present(&mut self, rl: &mut RaylibHandle, thread: &RaylibThread) {
        if self.texture.is_none() {
            let image = Image::gen_image_color(self.width, self.height, Color::BLACK);
            self.texture = rl.load_texture_from_image(thread, &image).ok();
        }
        let Some(texture) = self.texture.as_mut() else {
            return;
        };
        if texture.update_texture(self.color.as_flattened()).is_err() {
            return;
        }
        let mut d = rl.begin_drawing(thread);
        d.draw_texture(&*texture, 0, 0, Color::WHITE);
    }

    #[cfg(test)]
    pub fn covered_pixels(&self) -> usize {
        self.depth.iter().filter(|d| d.is_finite()).count()
    }

    #[cfg(test)]
    pub fn shader_at(&self, x: i32, y: i32) -> Option<ShaderType> {
        let index = (y * self.width + x) as usize;
        self.depth[index]
            .is_finite()
            .then_some(self.samples[index].shader)
    }
}

#[inline]
pub fn to_rgba(color: Vector3) -> [u8; 4] {
    [
        (color.x.clamp(0.0, 1.0) * 255.0) as u8,
        (color.y.clamp(0.0, 1.0) * 255.0) as u8,
        (color.z.clamp(0.0, 1.0) * 255.0) as u8,
        255,
    ]
}
