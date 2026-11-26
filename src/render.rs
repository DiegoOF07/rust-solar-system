// render.rs
// Pipeline de rasterizado: clip space, recorte contra el plano cercano y sombreado solo de lo visible

use crate::Uniforms;
use crate::fragment::Fragment;
use crate::framebuffer::{Framebuffer, Sample, to_rgba};
use crate::light::Light;
use crate::matrix::multiply_matrix_vector4;
use crate::shader_system::{ShaderType, apply_shader};
use crate::vertex::Vertex;
use raylib::prelude::*;
use rayon::prelude::*;

/// Alto de las franjas de pantalla que se rasterizan en paralelo
const BAND_HEIGHT: usize = 16;
/// Luz mínima para que la cara oscura no quede negra
const AMBIENT: f32 = 0.15;
/// Gris que los shaders modulan con la iluminación
const BASE_GRAY: f32 = 0.5;

/// Vértice en clip space, antes de dividir entre w
#[derive(Clone, Copy, Debug)]
pub struct ClipVertex {
    pub clip: [f32; 4],
    pub model_pos: Vector3,
    pub world_pos: Vector3,
    pub normal: Vector3,
}

impl ClipVertex {
    /// Distancia con signo al plano cercano (z + w >= 0 está delante)
    fn near_distance(&self) -> f32 {
        self.clip[2] + self.clip[3]
    }

    fn lerp(&self, other: &Self, t: f32) -> Self {
        let mut clip = [0.0; 4];
        for (i, c) in clip.iter_mut().enumerate() {
            *c = self.clip[i] + (other.clip[i] - self.clip[i]) * t;
        }
        ClipVertex {
            clip,
            model_pos: self.model_pos.lerp(other.model_pos, t),
            world_pos: self.world_pos.lerp(other.world_pos, t),
            normal: self.normal.lerp(other.normal, t),
        }
    }
}

/// Vértice ya proyectado a píxeles
#[derive(Clone, Copy, Debug)]
pub struct ScreenVertex {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub model_pos: Vector3,
    pub world_pos: Vector3,
    pub normal: Vector3,
}

#[derive(Clone, Copy, Debug)]
pub struct ScreenTriangle {
    pub v: [ScreenVertex; 3],
    pub shader: ShaderType,
}

impl ScreenTriangle {
    /// Doble del área con signo; el signo indica hacia dónde mira la cara
    fn signed_area(&self) -> f32 {
        let [a, b, c] = &self.v;
        (b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)
    }

    /// Caja del triángulo recortada a la pantalla: (x0, x1, y0, y1), inclusivos
    fn screen_bounds(&self, width: usize, height: usize) -> Option<(usize, usize, usize, usize)> {
        let xs = self.v.map(|v| v.x);
        let ys = self.v.map(|v| v.y);
        let min_x = xs.iter().copied().fold(f32::INFINITY, f32::min).floor();
        let max_x = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max).ceil();
        let min_y = ys.iter().copied().fold(f32::INFINITY, f32::min).floor();
        let max_y = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max).ceil();
        let (w, h) = ((width - 1) as f32, (height - 1) as f32);
        if max_x < 0.0 || max_y < 0.0 || min_x > w || min_y > h {
            return None;
        }
        Some((
            min_x.clamp(0.0, w) as usize,
            max_x.clamp(0.0, w) as usize,
            min_y.clamp(0.0, h) as usize,
            max_y.clamp(0.0, h) as usize,
        ))
    }
}

/// Polígono recortado de hasta 4 vértices, sin reservar memoria
pub struct ClipPolygon {
    verts: [ClipVertex; 4],
    len: usize,
}

impl ClipPolygon {
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[ClipVertex] {
        &self.verts[..self.len]
    }

    fn push(&mut self, v: ClipVertex) {
        self.verts[self.len] = v;
        self.len += 1;
    }
}

/// Recorta un triángulo contra el plano cercano (Sutherland-Hodgman).
/// Devuelve 0, 3 o 4 vértices, todos delante de la cámara.
pub fn clip_near(tri: &[ClipVertex; 3]) -> ClipPolygon {
    let mut out = ClipPolygon {
        verts: [tri[0]; 4],
        len: 0,
    };
    for i in 0..3 {
        let a = &tri[i];
        let b = &tri[(i + 1) % 3];
        let (da, db) = (a.near_distance(), b.near_distance());
        if da >= 0.0 {
            out.push(*a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            out.push(a.lerp(b, da / (da - db)));
        }
    }
    out
}

fn to_clip(vertex: &Vertex, u: &Uniforms) -> ClipVertex {
    let p = Vector4::new(vertex.position.x, vertex.position.y, vertex.position.z, 1.0);
    let world = multiply_matrix_vector4(&u.model_matrix, &p);
    let view = multiply_matrix_vector4(&u.view_matrix, &world);
    let clip = multiply_matrix_vector4(&u.projection_matrix, &view);
    let n = Vector4::new(vertex.normal.x, vertex.normal.y, vertex.normal.z, 0.0);
    let n = multiply_matrix_vector4(&u.model_matrix, &n);
    ClipVertex {
        clip: [clip.x, clip.y, clip.z, clip.w],
        model_pos: vertex.position,
        world_pos: Vector3::new(world.x, world.y, world.z),
        normal: Vector3::new(n.x, n.y, n.z).normalized(),
    }
}

fn to_screen(v: &ClipVertex, viewport: &Matrix) -> ScreenVertex {
    let w = v.clip[3];
    let ndc = Vector4::new(v.clip[0] / w, v.clip[1] / w, v.clip[2] / w, 1.0);
    let s = multiply_matrix_vector4(viewport, &ndc);
    ScreenVertex {
        x: s.x,
        y: s.y,
        z: s.z,
        model_pos: v.model_pos,
        world_pos: v.world_pos,
        normal: v.normal,
    }
}

/// Un objeto a dibujar en el cuadro
pub struct Draw<'a> {
    pub vertices: &'a [Vertex],
    pub uniforms: &'a Uniforms,
    pub shader: ShaderType,
    pub cull_backfaces: bool,
}

/// Transforma, recorta y proyecta los triángulos de todos los objetos en una sola pasada paralela
pub fn project(draws: &[Draw]) -> Vec<ScreenTriangle> {
    draws
        .par_iter()
        .flat_map(|draw| {
            draw.vertices
                .par_chunks_exact(3)
                .with_min_len(64)
                .flat_map_iter(|tri| {
                    let clip = [
                        to_clip(&tri[0], draw.uniforms),
                        to_clip(&tri[1], draw.uniforms),
                        to_clip(&tri[2], draw.uniforms),
                    ];
                    let poly = clip_near(&clip);
                    let mut fan = [None, None];
                    if !poly.is_empty() {
                        let v = poly.as_slice();
                        let first = to_screen(&v[0], &draw.uniforms.viewport_matrix);
                        let mut prev = to_screen(&v[1], &draw.uniforms.viewport_matrix);
                        // Un polígono recortado de 4 vértices se parte en abanico
                        for (slot, next) in fan.iter_mut().zip(&v[2..]) {
                            let next = to_screen(next, &draw.uniforms.viewport_matrix);
                            let t = ScreenTriangle {
                                v: [first, prev, next],
                                shader: draw.shader,
                            };
                            if !draw.cull_backfaces || t.signed_area() > 0.0 {
                                *slot = Some(t);
                            }
                            prev = next;
                        }
                    }
                    fan.into_iter().flatten()
                })
        })
        .collect()
}

/// Primera pasada: por píxel, guarda solo el fragmento más cercano y su iluminación
pub fn rasterize(fb: &mut Framebuffer, tris: &[ScreenTriangle], light: &Light) {
    let (width, height) = (fb.width as usize, fb.height as usize);
    let bounds: Vec<_> = tris
        .iter()
        .map(|t| t.screen_bounds(width, height))
        .collect();

    // Cada triángulo se anota en las franjas que toca
    let mut bins = vec![Vec::new(); height.div_ceil(BAND_HEIGHT)];
    for (i, b) in bounds.iter().enumerate() {
        if let Some((_, _, y0, y1)) = b {
            for bin in &mut bins[y0 / BAND_HEIGHT..=y1 / BAND_HEIGHT] {
                bin.push(i);
            }
        }
    }

    let (depth, samples) = fb.depth_and_samples_mut();
    depth
        .par_chunks_mut(BAND_HEIGHT * width)
        .zip(samples.par_chunks_mut(BAND_HEIGHT * width))
        .zip(bins.par_iter())
        .enumerate()
        .for_each(|(band, ((depth, samples), bin))| {
            let band_y0 = band * BAND_HEIGHT;
            let band_y1 = band_y0 + depth.len() / width - 1;
            for &i in bin {
                let (x0, x1, y0, y1) = bounds[i].expect("solo se anotan triángulos en pantalla");
                let tri = &tris[i];
                let area = tri.signed_area();
                if area.abs() < 1e-10 {
                    continue;
                }
                let [a, b, c] = &tri.v;
                for y in y0.max(band_y0)..=y1.min(band_y1) {
                    let py = y as f32 + 0.5;
                    for x in x0..=x1 {
                        let px = x as f32 + 0.5;
                        let w0 = ((b.x - px) * (c.y - py) - (c.x - px) * (b.y - py)) / area;
                        let w1 = ((c.x - px) * (a.y - py) - (a.x - px) * (c.y - py)) / area;
                        let w2 = 1.0 - w0 - w1;
                        if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                            continue;
                        }
                        let z = w0 * a.z + w1 * b.z + w2 * c.z;
                        let index = (y - band_y0) * width + x;
                        if z >= depth[index] {
                            continue;
                        }
                        depth[index] = z;
                        let normal = (a.normal * w0 + b.normal * w1 + c.normal * w2).normalized();
                        let world = a.world_pos * w0 + b.world_pos * w1 + c.world_pos * w2;
                        let to_light = (light.position - world).normalized();
                        samples[index] = Sample {
                            model_pos: a.model_pos * w0 + b.model_pos * w1 + c.model_pos * w2,
                            intensity: AMBIENT + (1.0 - AMBIENT) * normal.dot(to_light).max(0.0),
                            shader: tri.shader,
                        };
                    }
                }
            }
        });
}

/// Segunda pasada: corre el shader una vez por píxel visible
pub fn shade(fb: &mut Framebuffer, uniforms: &Uniforms) {
    let width = fb.width as usize;
    let (color, depth, samples) = fb.color_depth_samples_mut();
    color
        .par_chunks_mut(width)
        .zip(depth.par_chunks(width))
        .zip(samples.par_chunks(width))
        .for_each(|((row, depth), samples)| {
            for ((pixel, depth), sample) in row.iter_mut().zip(depth).zip(samples) {
                if depth.is_finite() {
                    let gray = BASE_GRAY * sample.intensity;
                    let fragment = Fragment {
                        color: Vector3::new(gray, gray, gray),
                        world_position: sample.model_pos,
                    };
                    *pixel = to_rgba(apply_shader(&fragment, uniforms, sample.shader));
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framebuffer::Framebuffer;
    use crate::light::Light;
    use crate::matrix::{
        create_model_matrix, create_projection_matrix, create_view_matrix, create_viewport_matrix,
    };
    use crate::obj::Obj;
    use crate::shader_system::ShaderType;
    use std::time::{Duration, Instant};

    const W: i32 = 320;
    const H: i32 = 240;

    fn uniforms(model: Matrix, eye: Vector3, target: Vector3) -> crate::Uniforms {
        crate::Uniforms {
            model_matrix: model,
            view_matrix: create_view_matrix(eye, target, Vector3::new(0.0, 1.0, 0.0)),
            projection_matrix: create_projection_matrix(
                std::f32::consts::PI / 3.0,
                W as f32 / H as f32,
                0.1,
                100.0,
            ),
            viewport_matrix: create_viewport_matrix(0.0, 0.0, W as f32, H as f32),
            time: 1.0,
        }
    }

    fn clip_vertex(x: f32, y: f32, z: f32, w: f32) -> ClipVertex {
        ClipVertex {
            clip: [x, y, z, w],
            model_pos: Vector3::new(x, y, z),
            world_pos: Vector3::new(x, y, z),
            normal: Vector3::new(0.0, 0.0, 1.0),
        }
    }

    fn sphere() -> Vec<crate::vertex::Vertex> {
        Obj::load("./models/sphere.obj")
            .expect("sphere.obj")
            .get_vertex_array()
    }

    #[test]
    fn clip_near_keeps_a_triangle_in_front_of_the_camera() {
        let tri = [
            clip_vertex(0.0, 0.0, 1.0, 2.0),
            clip_vertex(1.0, 0.0, 1.0, 2.0),
            clip_vertex(0.0, 1.0, 1.0, 2.0),
        ];
        let out = clip_near(&tri);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn clip_near_drops_a_triangle_behind_the_camera() {
        let tri = [
            clip_vertex(0.0, 0.0, -3.0, 1.0),
            clip_vertex(1.0, 0.0, -3.0, 1.0),
            clip_vertex(0.0, 1.0, -3.0, 1.0),
        ];
        assert!(clip_near(&tri).is_empty());
    }

    #[test]
    fn clip_near_cuts_a_crossing_triangle_and_leaves_no_vertex_behind() {
        // Un vértice detrás (z + w < 0) y dos delante: queda un cuadrilátero
        let tri = [
            clip_vertex(0.0, 0.0, -2.0, 1.0),
            clip_vertex(1.0, 0.0, 1.0, 2.0),
            clip_vertex(0.0, 1.0, 1.0, 2.0),
        ];
        let out = clip_near(&tri);
        assert_eq!(out.len(), 4);
        for v in out.as_slice() {
            assert!(
                v.clip[2] + v.clip[3] >= -1e-4,
                "vértice detrás del plano: {:?}",
                v.clip
            );
        }

        // Dos detrás y uno delante: queda un triángulo más pequeño
        let tri = [
            clip_vertex(0.0, 0.0, -2.0, 1.0),
            clip_vertex(1.0, 0.0, -2.0, 1.0),
            clip_vertex(0.0, 1.0, 1.0, 2.0),
        ];
        assert_eq!(clip_near(&tri).len(), 3);
    }

    #[test]
    fn rasterizing_a_huge_triangle_only_touches_the_screen() {
        // Coordenadas de millones de píxeles, como las que dejaba un vértice pegado a la cámara
        let v = |x: f32, y: f32| ScreenVertex {
            x,
            y,
            z: 0.5,
            model_pos: Vector3::zero(),
            world_pos: Vector3::zero(),
            normal: Vector3::new(0.0, 0.0, 1.0),
        };
        let tri = ScreenTriangle {
            v: [v(-5.0e6, -5.0e6), v(5.0e6, -5.0e6), v(0.0, 5.0e6)],
            shader: ShaderType::Rocky,
        };
        let mut fb = Framebuffer::new(W, H);
        let start = Instant::now();
        rasterize(&mut fb, &[tri], &Light::new(Vector3::zero()));
        assert!(start.elapsed() < Duration::from_millis(200));
        assert_eq!(fb.covered_pixels(), (W * H) as usize);
    }

    #[test]
    fn the_nearest_surface_wins_whatever_the_draw_order() {
        let v = |x: f32, y: f32, z: f32| ScreenVertex {
            x,
            y,
            z,
            model_pos: Vector3::zero(),
            world_pos: Vector3::zero(),
            normal: Vector3::new(0.0, 0.0, 1.0),
        };
        let full = |z: f32, shader: ShaderType| ScreenTriangle {
            v: [
                v(-1000.0, -1000.0, z),
                v(1000.0, -1000.0, z),
                v(0.0, 1000.0, z),
            ],
            shader,
        };
        for order in [
            [full(0.2, ShaderType::Rocky), full(0.8, ShaderType::Ocean)],
            [full(0.8, ShaderType::Ocean), full(0.2, ShaderType::Rocky)],
        ] {
            let mut fb = Framebuffer::new(W, H);
            rasterize(&mut fb, &order, &Light::new(Vector3::zero()));
            assert_eq!(fb.shader_at(W / 2, H / 2), Some(ShaderType::Rocky));
        }
    }

    #[test]
    fn backface_culling_keeps_only_the_half_facing_the_camera() {
        let eye = Vector3::new(0.0, 0.0, 5.0);
        let u = uniforms(
            create_model_matrix(Vector3::zero(), 1.0, Vector3::zero()),
            eye,
            Vector3::zero(),
        );
        let mut all = Vec::new();
        all.extend(project(&[Draw {
            vertices: &sphere(),
            uniforms: &u,
            shader: ShaderType::Rocky,
            cull_backfaces: false,
        }]));
        let mut front = Vec::new();
        front.extend(project(&[Draw {
            vertices: &sphere(),
            uniforms: &u,
            shader: ShaderType::Rocky,
            cull_backfaces: true,
        }]));

        assert!(
            front.len() < all.len() * 6 / 10,
            "{} de {}",
            front.len(),
            all.len()
        );
        assert!(
            front.len() > all.len() * 4 / 10,
            "{} de {}",
            front.len(),
            all.len()
        );
        for t in &front {
            let centroid_z = (t.v[0].world_pos.z + t.v[1].world_pos.z + t.v[2].world_pos.z) / 3.0;
            assert!(centroid_z > -0.35, "cara trasera visible: z = {centroid_z}");
        }
    }

    #[test]
    fn a_planet_grazing_the_camera_renders_in_bounded_time() {
        // La cámara a 1.05 radios mirando de lado: parte de la esfera queda detrás de ella
        let eye = Vector3::new(1.05, 0.0, 0.0);
        let u = uniforms(
            create_model_matrix(Vector3::zero(), 1.0, Vector3::zero()),
            eye,
            Vector3::new(1.05, 0.0, -10.0),
        );
        let mut fb = Framebuffer::new(W, H);
        let start = Instant::now();
        let mut tris = Vec::new();
        tris.extend(project(&[Draw {
            vertices: &sphere(),
            uniforms: &u,
            shader: ShaderType::Rocky,
            cull_backfaces: true,
        }]));
        rasterize(&mut fb, &tris, &Light::new(Vector3::zero()));
        shade(&mut fb, &u);
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "{:?}",
            start.elapsed()
        );
    }

    #[test]
    fn a_planet_in_front_of_the_camera_covers_the_center_of_the_screen() {
        let u = uniforms(
            create_model_matrix(Vector3::zero(), 1.0, Vector3::zero()),
            Vector3::new(0.0, 0.0, 4.0),
            Vector3::zero(),
        );
        let mut fb = Framebuffer::new(W, H);
        let mut tris = Vec::new();
        tris.extend(project(&[Draw {
            vertices: &sphere(),
            uniforms: &u,
            shader: ShaderType::Rocky,
            cull_backfaces: true,
        }]));
        rasterize(&mut fb, &tris, &Light::new(Vector3::new(10.0, 0.0, 10.0)));
        assert_eq!(fb.shader_at(W / 2, H / 2), Some(ShaderType::Rocky));
        assert_eq!(fb.shader_at(2, 2), None);
    }
}
