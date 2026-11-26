# Sistema solar en Rust

> Sistema solar interactivo con un motor de render 3D propio en Rust y Raylib

[![CI](https://github.com/DiegoOF07/rust-solar-system/actions/workflows/ci.yml/badge.svg)](https://github.com/DiegoOF07/rust-solar-system/actions/workflows/ci.yml)

[![Video del sistema solar en ejecución](https://img.youtube.com/vi/KpaX5xlLnqM/maxresdefault.jpg)](https://www.youtube.com/watch?v=KpaX5xlLnqM)

*Clic en la imagen para ver el video.*

Un rasterizador por software escrito desde cero: transformación de vértices, recorte, rasterización de triángulos, prueba de profundidad y shaders de fragmentos, todo en Rust y en CPU. Raylib solo abre la ventana y muestra el cuadro ya calculado.

## Qué hace

- **El sol y 8 planetas** en órbita, cada uno con su propio shader procedural: estrella con manchas y destellos, y planetas rocoso, de lava, tipo Tierra con nubes, oceánico, gigante gaseoso, desértico, gigante con bandas y helado.
- **Saltos entre cuerpos:** la cámara viaja al planeta elegido y lo sigue en su órbita.
- **Nave espacial** con un shader metálico, cargada desde un modelo `.obj`.
- **Texturas sin imágenes:** todo sale de ruido procedural (Simplex y Voronoi).
- **Cámara libre** en 3D, órbitas visibles y fondo de estrellas.

## Stack

Rust (edición 2024) · Raylib 5.5 (ventana y presentación) · Rayon (paralelismo) · tobj (carga de modelos)

## Cómo funciona

```
obj.rs, vertex.rs           modelos y vértices
matrix.rs, camera.rs        matrices de modelo, vista y proyección
render.rs                   recorte, rasterizado y sombreado, en paralelo con Rayon
shader_system.rs, noise.rs  shaders por tipo de cuerpo y ruido procedural
framebuffer.rs              buffers de color y profundidad
solar_system.rs, warp.rs    órbitas, cuerpos y saltos de cámara
```

Cada cuadro pasa por tres etapas:

1. **Geometría.** Los vértices de todos los cuerpos se llevan a clip space, los triángulos que cruzan el plano cercano de la cámara se recortan y las caras traseras de las esferas se descartan.
2. **Visibilidad.** La pantalla se divide en franjas de 16 filas que se rasterizan en paralelo. Por cada píxel se guarda solo el fragmento más cercano.
3. **Sombreado.** El shader corre una sola vez por píxel visible, así que el costo depende del tamaño de la ventana y no de cuántas superficies se tapan entre sí.

## Cómo correrlo

Requiere Rust estable y las dependencias de compilación de Raylib (CMake y un compilador de C). Se corre desde la raíz del repo, porque los modelos se cargan de `models/`.

```bash
cargo run --release
```

| Tecla | Acción |
|---|---|
| W A S D | rotar la cámara |
| Q / E | desplazamiento horizontal |
| R / F | desplazamiento vertical |
| ↑ / ↓ | zoom |
| Z / X | avanzar o retroceder |
| TAB | saltar al siguiente cuerpo |
| 1 – 9 | saltar a un cuerpo (1 es el sol) |
| 0 | dejar de seguir al cuerpo |
| O | mostrar u ocultar las órbitas |
| V | mostrar u ocultar la nave |
| Esc | salir |

## Rendimiento

`cargo run --release -- --bench` renderiza la misma escena desde varias distancias a un planeta, sin abrir ventana, y reporta el tiempo por cuadro. A 1300×900 en un Intel i5-10400 (12 hilos):

| Vista | Antes | Ahora |
|---|---|---|
| Vista general | 106 ms | 17 ms (~58 FPS) |
| A 4 radios del planeta | 167 ms | 19 ms (~52 FPS) |
| A 2 radios | 168 ms | 29 ms (~34 FPS) |
| A 1.5 radios | 261 ms | 34 ms (~29 FPS) |
| Cámara junto al planeta | no terminaba | 21 ms (~47 FPS) |

La primera versión se congelaba al acercarse: cuando un cuerpo quedaba junto o detrás de la cámara, sus vértices se proyectaban a millones de píxeles y el rasterizador recorría todo ese rectángulo. Ahora los triángulos se recortan contra el plano cercano y la caja de cada uno se limita a la pantalla. El resto de la mejora viene de sombrear solo lo visible y de no reservar memoria por triángulo.

Con un planeta llenando la pantalla, el límite son los shaders: cada píxel calcula varias capas de ruido.

## Decisiones y aprendizajes

- **Render en CPU a propósito.** Escribir el pipeline a mano (proyección, recorte, rasterización, z-buffer y shaders) era el objetivo del curso. Una GPU lo haría más rápido, pero no enseñaría cómo funciona.
- **Recortar antes de dividir entre w.** Un vértice detrás de la cámara tiene w negativo, y al proyectarlo cae en cualquier parte de la pantalla. Hay que recortar en clip space, no en píxeles.
- **Sombrear solo lo visible.** Separar la visibilidad del sombreado acota el costo del shader al número de píxeles, que es lo que hace usable la cámara de cerca.
- **Un shader por tipo de cuerpo** en lugar de texturas. Así las superficies se animan con el tiempo y no hay archivos de imagen que cargar.

Proyecto 3 del curso Gráficas por Computadora, UVG, 2025.

## Licencia

[MIT](LICENSE)
