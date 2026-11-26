// main.rs
// Sistema solar con un rasterizador por software: ventana, controles y modo de medición

mod camera;
mod collision;
mod fragment;
mod framebuffer;
mod light;
mod matrix;
mod noise;
mod obj;
mod orbit_renderer;
mod render;
mod shader_system;
mod skybox;
mod solar_system;
mod spaceship;
mod vertex;
mod warp;

use camera::Camera;
use collision::CollisionSystem;
use framebuffer::Framebuffer;
use light::Light;
use matrix::{
    create_model_matrix, create_projection_matrix, create_view_matrix, create_viewport_matrix,
};
use obj::Obj;
use orbit_renderer::OrbitRenderer;
use raylib::prelude::*;
use skybox::Skybox;
use solar_system::SolarSystem;
use spaceship::Spaceship;
use std::f32::consts::PI;
use std::time::Instant;
use vertex::Vertex;
use warp::WarpSystem;

const WIDTH: i32 = 1300;
const HEIGHT: i32 = 900;

pub struct Uniforms {
    pub model_matrix: Matrix,
    pub view_matrix: Matrix,
    pub projection_matrix: Matrix,
    pub viewport_matrix: Matrix,
    pub time: f32,
}

/// Todo lo que se dibuja en un cuadro
struct Scene {
    system: SolarSystem,
    sphere: Vec<Vertex>,
    spaceship: Spaceship,
    ship_position: Vector3,
    ship_rotation: Vector3,
    skybox: Skybox,
    orbits: OrbitRenderer,
    light: Light,
    projection: Matrix,
    viewport: Matrix,
}

impl Scene {
    fn load() -> Self {
        let sphere = Obj::load("./models/sphere.obj")
            .expect("No se pudo cargar models/sphere.obj; corre el programa desde la raíz del repo")
            .get_vertex_array();
        let spaceship = Spaceship::load("./models/spaceship.obj", 0.3).expect(
            "No se pudo cargar models/spaceship.obj; corre el programa desde la raíz del repo",
        );
        Scene {
            system: SolarSystem::create_basic_system(),
            sphere,
            spaceship,
            ship_position: Vector3::new(-15.0, -5.0, -20.0),
            ship_rotation: Vector3::new(0.0, PI + 0.785, PI),
            skybox: Skybox::new(WIDTH, HEIGHT, 800),
            orbits: OrbitRenderer::new(64),
            // La luz está en el sol
            light: Light::new(Vector3::zero()),
            projection: create_projection_matrix(
                PI / 3.0,
                WIDTH as f32 / HEIGHT as f32,
                0.1,
                100.0,
            ),
            viewport: create_viewport_matrix(0.0, 0.0, WIDTH as f32, HEIGHT as f32),
        }
    }

    fn bodies(&self) -> Vec<(Vector3, f32)> {
        self.system
            .objects
            .iter()
            .map(|o| (o.position, o.scale))
            .collect()
    }
}

struct Toggles {
    orbits: bool,
    ship: bool,
}

/// Descarta lo que está lejos o claramente detrás de la cámara
fn is_visible(pos: Vector3, scale: f32, eye: Vector3, target: Vector3) -> bool {
    let to_obj = pos - eye;
    let dist = to_obj.length();
    if dist > 80.0 {
        return false;
    }
    to_obj.dot(target - eye) > -scale * 2.0 || dist < scale * 5.0
}

fn render_frame(
    fb: &mut Framebuffer,
    scene: &Scene,
    eye: Vector3,
    target: Vector3,
    time: f32,
    toggles: &Toggles,
) {
    let view = create_view_matrix(eye, target, Vector3::new(0.0, 1.0, 0.0));
    let uniforms = |model_matrix| Uniforms {
        model_matrix,
        view_matrix: view,
        projection_matrix: scene.projection,
        viewport_matrix: scene.viewport,
        time,
    };

    fb.clear();
    scene.skybox.render(fb, time);
    if toggles.orbits {
        for obj in &scene.system.objects {
            if let Some(parent) = obj.parent_index {
                let center = scene.system.objects[parent].position;
                scene.orbits.render_orbit(
                    fb,
                    center,
                    obj.orbit_radius,
                    &view,
                    &scene.projection,
                    &scene.viewport,
                );
            }
        }
    }

    let mut bodies = Vec::new();
    for obj in &scene.system.objects {
        if is_visible(obj.position, obj.scale, eye, target) {
            let model = create_model_matrix(obj.position, obj.scale, obj.rotation);
            bodies.push((uniforms(model), obj.shader_type));
        }
    }
    let ship = (toggles.ship
        && is_visible(scene.ship_position, scene.spaceship.scale, eye, target))
    .then(|| {
        uniforms(create_model_matrix(
            scene.ship_position,
            scene.spaceship.scale,
            scene.ship_rotation,
        ))
    });

    let mut draws: Vec<render::Draw> = bodies
        .iter()
        .map(|(u, shader)| render::Draw {
            vertices: &scene.sphere,
            uniforms: u,
            shader: *shader,
            cull_backfaces: true,
        })
        .collect();
    if let Some(u) = &ship {
        // La malla de la nave no tiene un orden de vértices consistente, así que no se descartan caras
        draws.push(render::Draw {
            vertices: scene.spaceship.get_vertices(),
            uniforms: u,
            shader: scene.spaceship.shader_type,
            cull_backfaces: false,
        });
    }
    let triangles = render::project(&draws);
    render::rasterize(fb, &triangles, &scene.light);
    render::shade(fb, &uniforms(Matrix::identity()));
}

fn main() {
    if std::env::args().any(|a| a == "--bench") {
        bench();
        return;
    }

    let (mut window, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("Sistema solar")
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();
    window.set_target_fps(60);

    let mut scene = Scene::load();
    let mut framebuffer = Framebuffer::new(WIDTH, HEIGHT);
    framebuffer.set_background_color(Color::new(2, 2, 8, 255));
    let mut camera = Camera::new(Vector3::new(0.0, 25.0, 40.0), Vector3::zero());
    let mut warp = WarpSystem::new();
    let mut toggles = Toggles {
        orbits: true,
        ship: true,
    };

    print_controls();

    while !window.window_should_close() {
        let delta_time = window.get_frame_time();
        let time = window.get_time() as f32;

        scene.system.update(delta_time);
        let bodies = scene.bodies();

        handle_warp_input(&window, &mut warp, &camera, bodies.len());
        if window.is_key_pressed(KeyboardKey::KEY_O) {
            toggles.orbits = !toggles.orbits;
        }
        if window.is_key_pressed(KeyboardKey::KEY_V) {
            toggles.ship = !toggles.ship;
        }

        if let Some((eye, target)) = warp.update(delta_time, &bodies, camera.eye, camera.target) {
            camera.set_position(eye, target);
        }
        if !warp.is_warping() {
            camera.process_input(&window);
        }
        camera.eye = CollisionSystem::check_and_resolve(camera.eye, &bodies, 0.5);

        render_frame(
            &mut framebuffer,
            &scene,
            camera.eye,
            camera.target,
            time,
            &toggles,
        );
        framebuffer.present(&mut window, &thread);
    }
}

/// TAB: siguiente cuerpo, 1-9: cuerpo por número (1 es el sol), 0: dejar de seguir
fn handle_warp_input(window: &RaylibHandle, warp: &mut WarpSystem, camera: &Camera, count: usize) {
    if window.is_key_pressed(KeyboardKey::KEY_TAB) {
        warp.warp_to(warp.next_body(count), camera.eye, camera.target);
    }
    if window.is_key_pressed(KeyboardKey::KEY_ZERO) {
        warp.stop_following();
    }
    let number_keys = [
        KeyboardKey::KEY_ONE,
        KeyboardKey::KEY_TWO,
        KeyboardKey::KEY_THREE,
        KeyboardKey::KEY_FOUR,
        KeyboardKey::KEY_FIVE,
        KeyboardKey::KEY_SIX,
        KeyboardKey::KEY_SEVEN,
        KeyboardKey::KEY_EIGHT,
        KeyboardKey::KEY_NINE,
    ];
    for (body, key) in number_keys.into_iter().enumerate() {
        if body < count && window.is_key_pressed(key) {
            warp.warp_to(body, camera.eye, camera.target);
        }
    }
}

/// `cargo run --release -- --bench`: mide el tiempo por cuadro acercándose a un planeta, sin abrir ventana
fn bench() {
    const FRAMES: u32 = 10;
    const PLANET: usize = 3;

    let mut scene = Scene::load();
    // Órbitas fijas para que cada corrida mida la misma escena
    for (i, obj) in scene.system.objects.iter_mut().enumerate() {
        obj.orbit_angle = i as f32 * 0.7;
    }
    scene.system.update(0.0);
    let mut fb = Framebuffer::new(WIDTH, HEIGHT);
    let toggles = Toggles {
        orbits: true,
        ship: true,
    };

    let planet = scene.system.objects[PLANET].position;
    let radius = scene.system.objects[PLANET].scale;
    let mut views = vec![(
        "vista general".to_string(),
        Vector3::new(0.0, 25.0, 40.0),
        Vector3::zero(),
    )];
    for d in [8.0, 4.0, 2.0, 1.5, 1.2, 1.05] {
        views.push((
            format!("a {d} radios del planeta"),
            planet + Vector3::new(0.0, 0.2 * radius, d * radius),
            planet,
        ));
    }
    let side = planet + Vector3::new(1.1 * radius, 0.0, 0.0);
    views.push((
        "rozando el planeta".to_string(),
        side,
        side + Vector3::new(0.0, 0.0, -10.0),
    ));

    println!(
        "{WIDTH}x{HEIGHT}, {} hilos, promedio de {FRAMES} cuadros",
        rayon::current_num_threads()
    );
    for (name, eye, target) in views {
        let start = Instant::now();
        for frame in 0..FRAMES {
            render_frame(&mut fb, &scene, eye, target, frame as f32 / 60.0, &toggles);
        }
        let ms = start.elapsed().as_secs_f64() * 1000.0 / FRAMES as f64;
        println!("{name:<26} {ms:>7.1} ms  ({:.0} FPS)", 1000.0 / ms);
    }
}

fn print_controls() {
    println!("W A S D   rotar la cámara");
    println!("Q / E     desplazamiento horizontal   R / F  vertical");
    println!("↑ / ↓     zoom                        Z / X  avanzar o retroceder");
    println!("TAB       saltar al siguiente cuerpo  1-9    saltar a un cuerpo (1 es el sol)");
    println!("0         dejar de seguir al cuerpo");
    println!("O         órbitas                     V      nave");
    println!("Esc       salir");
}
