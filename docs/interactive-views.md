# Vista interactiva de la nave (mejora opcional posterior al Lab 5)

Esta funcionalidad **no es un requisito** del Ing. Dennis para los Laboratorios
4 o 5; es una ampliación para explorar la nave `NaveBimotor` desde cualquier
ángulo con los tres modos de rasterización ya entregados. Vive en la rama
`feature/lab5-interactive-view` y no altera los oráculos de las pruebas, los
CSV del benchmark ni las capturas anteriores.

## Uso

```bash
cargo run --release -- --scene ship --interactive --mode wireframe
cargo run --release -- --scene ship --interactive --mode barycentric
cargo run --release -- --scene ship --interactive --mode scanline
cargo run --release -- --view top --mode scanline                     # vista fija (sin teclado)
cargo run --release -- --interactive --view side --screenshot vista.png
cargo run --release -- --help
```

| Tecla | Acción |
|---|---|
| `←` / `→` | Orbitar alrededor de la nave (azimut) |
| `↑` / `↓` | Inclinar la vista (elevación, limitada a ±89°) |
| `Q` / `E` | Alejar / acercar (zoom ortográfico, entre 0.25× y 8×) |
| `1` / `2` / `3` | Wireframe / FilledBarycentric / FilledScanline, **sin reiniciar la cámara** |
| `R` | Restaurar el preset inicial (el de `--view`, oblicuo por defecto) y el zoom |
| `T` / `F` / `S` | Vista superior / frontal / lateral (conservan el zoom) |
| `ESC` o cerrar | Salir |

Las teclas sostenidas se integran con el tiempo entre cuadros (`dt`, acotado a
50 ms para que una pausa no teletransporte la cámara); la velocidad angular es
1.6 rad/s y el zoom cambia 2× por segundo. `cargo run` sin argumentos y
`--scene triangles` siguen exactamente igual que antes.

## Cómo funciona (todo en CPU)

```text
OBJ (una carga) → Vertex (posición original intacta)
   → OrbitCamera {target = centro del bounding box, azimuth, elevation, zoom}
   → base ortonormal right/up/forward (productos cruzados, Y arriba)
   → world_to_view: (x_right, y_up, depth = coordenada sobre forward)
   → escala fija = (lado menor del viewport − margen) / (2·radio del modelo) × zoom
   → transformed_position = (cx + x·s, cy − y·s, depth)
   → [relleno] sombreado plano por cara (normal del modelo · luz direccional)
   → primitive_assembly → rasterize_with_mode (sin cambios)
   → Fragment.depth interpolado (pesos baricéntricos / plano del triángulo)
   → Framebuffer::point_depth (Z-buffer, el más cercano gana) o point (wireframe)
   → textura Raylib
```

- **Proyección ortográfica orbital**, sin perspectiva: el ojo está en una
  esfera alrededor del centro del modelo; la profundidad de vista es la
  coordenada a lo largo de `forward` (menor = más cerca). La escala se calcula
  **una vez** a partir del radio de la esfera envolvente, por lo que rotar no
  cambia el tamaño y el zoom es lo único que lo modifica. Con zoom alto la nave
  puede salir de la ventana a propósito.
- **Profundidad**: `fill_barycentric` interpola `z` con los pesos que ya
  calculaba; `fill_scanline` evalúa el plano del triángulo (`DepthPlane`) en
  cada centro. La cobertura `(x, y)` no cambia: las pruebas T01–T09 y el
  corpus LCG siguen comparando conjuntos idénticos. El `Framebuffer` conserva
  `point` (última escritura gana) para los laboratorios 4 y 5 y añade
  `point_depth`/`clear_depth`; los empates conservan el primer fragmento y los
  valores no finitos se descartan.
- **Sombreado**: tono uniforme por cara `ambiente + (1 − ambiente)·|n·l|` con
  normal en espacio del modelo y luz direccional fija. Es un **sombreado de
  demostración calculado**, no los materiales de Blender (el cargador solo lee
  posiciones). El wireframe mantiene contornos blancos y dibuja todas las
  aristas (sin eliminación de líneas ocultas).
- Baricéntrico y scanline siguen siendo implementaciones independientes;
  ambos producen exactamente la misma imagen sombreada y con profundidad
  (verificado píxel a píxel en pruebas y en las capturas).

## Módulos y pruebas

| Archivo | Responsabilidad |
|---|---|
| `src/pipeline/orbit_camera.rs` | `OrbitCamera`, `CameraBasis`, `ViewPreset` |
| `src/pipeline/screen_mapping.rs` | `model_bounds`, `OrbitView`, `project_orbit` (junto a `fit_to_viewport` del Lab 4) |
| `src/pipeline/render.rs` | `Projection::Orbit`, `RenderSettings`, `render_with_settings`, `render_interactive` |
| `src/pipeline/face_shading.rs` | `DirectionalLight`, `face_normal`, `intensity`, `shade_faces` |
| `src/rasterization/coverage.rs` | `orient_with_depth`, `DepthPlane` |
| `src/rendering/framebuffer.rs` | `point_depth`, `clear_depth`, `get_depth` |
| `src/app.rs` | `CameraAction`, `apply_input`, teclado, `--interactive`, `--view`, `--help` |
| `tests/interactive_*`, `orbit_*`, `depth_buffer_tests.rs`, `face_shading_tests.rs`, `camera_preset_tests.rs` | 51 pruebas nuevas |

## Evidencia

Misma vista oblicua (`--interactive --view oblique`) en los tres modos:
`docs/screenshots/lab5/04-ship-wireframe-oblique.png`,
`05-ship-barycentric-oblique.png`, `06-ship-scanline-oblique.png`. Las dos
imágenes rellenas son byte a byte iguales.

## Límites

Sin perspectiva, sin materiales MTL, sin eliminación de líneas ocultas en
wireframe, sin antialiasing. Los benchmarks y perfiles del Lab 5 no se
repitieron: miden la misma generación de fragmentos sobre la escena sintética
(sin cámara, sin profundidad ni sombreado), por lo que siguen siendo válidos
para esa carga; una comparación con el modo interactivo requeriría una corrida
nueva etiquetada como tal.
