# Proyecto 3 — Evidencia de geometría procedural y colocación (C76)

Este documento tiene una sección por rubro, cada una con su prueba, su captura y su comando. Las cuatro capturas de rubros puntuables son distintas entre sí, así que ninguna evidencia sirve para dos rubros:
- deformación (10);
- subdivisión (10);
- distribución (10);
- orientación (5).

Las capturas nuevas salen del framebuffer de CPU, a 800×600 por cuadro, y están en `docs/results/project3/geometry/`. En cada una, el cuadro de la izquierda es el «antes» y el de la derecha el «después».

Las cuatro pruebas de `tests/p3_procedural_scene_tests.rs` corren sobre los planetas tal como los construye la escena del Proyecto 3:

```bash
cargo test --release --test p3_procedural_scene_tests
```

## 1. Subdivisión procedural antes de rasterizar (10)

- **Qué es:**
  - La icosfera (`src/solar/planet/icosphere.rs`) parte del icosaedro. Cada nivel divide cada triángulo en cuatro, con los vértices nuevos sobre la esfera: `20·4ⁿ` triángulos.
  - Los niveles 2 a 5 se construyen una sola vez al cargar (`GeometryCache`), antes de cualquier rasterizado.
- **Prueba:** `subdivision_independent_of_relief`.
  - En los niveles 2–5 de `relief_mesh`, el número de triángulos es `20·4ⁿ` con el campo plano, con el del trébol y con el de Hearts.
  - Las mallas que usa la escena para los dos planetas con relieve tienen esos mismos conteos.
  - La topología la cubre `tests/p3_icosphere_topology_tests.rs`: Euler = 2, cada arista compartida por dos caras, enrollado hacia afuera y nivel 6 rechazado.
- **Captura:** [subdivision_l2_l5.png](geometry/subdivision_l2_l5.png). Es el trébol en alambre, sin modelos de superficie, con el nivel forzado.
  - A la izquierda, nivel 2: 320 triángulos del planeta (1214 en el cuadro con la nave).
  - A la derecha, nivel 5: 20 480 triángulos del planeta (21 374 en el cuadro).
- **Comando:**

```bash
cargo test --release --test p3_procedural_scene_tests subdivision_independent_of_relief
cargo test --release --test p3_icosphere_topology_tests
```

## 2. Deformación de la geometría con ruido (10)

- **Qué es:** `relief_mesh` coloca cada dirección `n` de la icosfera a `R · surface_radius(n)` del centro. `surface_radius` sale del campo de altura:
  - en el trébol, montañas con forma de trébol y lecho marino con fbm;
  - en Hearts, terrazas cuantizadas de fbm.
- **Prueba:** `relief_independent_of_subdivision`.
  - En cada nivel, cada vértice está a la altura del campo en su dirección (±1e-5).
  - Los vértices del nivel 2 aparecen bit a bit en el nivel 5: la altura no depende de la subdivisión.
  - **Caso negativo:** el campo `Flat` deja todos los vértices en el radio base (no cuenta como deformación), y el campo de Hearts sí deforma.
  - Ver también `tests/p3_relief_geometry_tests.rs`.
- **Captura:** [relief_flat_vs_field.png](geometry/relief_flat_vs_field.png). Es la vista del escenario K3 sin modelos de superficie.
  - A la izquierda, el trébol con `HeightField::Flat` (la esfera).
  - A la derecha, con su campo: la montaña y la isla se levantan sobre el horizonte.
- **Comando:**

```bash
cargo test --release --test p3_procedural_scene_tests relief_independent_of_subdivision
cargo test --release --test p3_relief_geometry_tests
```

## 3. Distribución procedural con semilla (10)

- **Qué es:** árboles y rocas en el trébol, y edificios en Hearts. Se colocan con la red de Fibonacci, un hash por semilla, una máscara de ruido, la pendiente, la tierra firme y un espaciado mínimo (`scatter`). La semilla es la del planeta.
- **Prueba:** `seed_distribution`.
  - Dos escenas con el mismo spec tienen exactamente los mismos sitios: más de 200 en el trébol y más de 400 en la ciudad.
  - Con la semilla + 1 en cada planeta, más del 90 % de los sitios cambian.
  - Ver también `tests/p3_scatter_seed_tests.rs` y `tests/p3_hearts_city_distribution_tests.rs`.
- **Captura:** [seed_7_vs_8.png](geometry/seed_7_vs_8.png). Es la vista del escenario K con la semilla 7 (izquierda) y la 8 (derecha): otras islas, otros árboles y otras rocas.
- **En vivo:** `cargo run --release -- --scene solar --set clover.seed=8`.
- **Comando:**

```bash
cargo test --release --test p3_procedural_scene_tests seed_distribution
cargo test --release --test p3_scatter_seed_tests --test p3_hearts_city_distribution_tests
```

## 4. Posición y orientación sobre la superficie al girar (5)

- **Qué es:**
  - Los modelos se hornean una vez en el espacio del planeta, con su «arriba» igual a la normal del campo en su pie.
  - Se dibujan con la matriz del planeta, así que su órbita y su giro los llevan consigo sin recalcular nada.
- **Prueba:** `orientation_on_rotation`.
  - En t = 0, 7.5 y 31 s, el «arriba» de la pirámide forma siempre el mismo ángulo con su radial (está de pie sobre su meseta).
  - Un edificio conserva su lugar en el marco del planeta.
  - Entre esos tiempos, el planeta se mueve y gira, y los dos giran con él.
  - Ver también `tests/p3_prop_sector_tests.rs`.
- **Captura:** [orientation_spin.png](geometry/orientation_spin.png). Muestra Hearts desde una cámara que sigue al planeta sin girar con él, a 15 s de distancia (1.5 rad de giro). La pirámide y la ciudad giran con la superficie y siguen perpendiculares a ella.
- **Comando:**

```bash
cargo test --release --test p3_procedural_scene_tests orientation_on_rotation
cargo test --release --test p3_prop_sector_tests
```

## 5. Normal del relieve

- **Qué es:** el sombreado del relieve usa la normal del campo, por diferencias finitas sobre la dirección (`NORMAL_EPS` = 1e-3). El parámetro `bump` la mezcla con la normal de la esfera: 0 da la normal de la esfera y 1 la del relieve.
- **Prueba:** `tests/p3_relief_normals_tests.rs`.
- **Captura:** [normals_bump_0_1.png](geometry/normals_bump_0_1.png). Es el trébol en el terminador, con luz rasante y sin modelos.
  - A la izquierda, `bump` = 0: la montaña queda tan oscura como la esfera.
  - A la derecha, `bump` = 1: la normal del campo enciende las laderas que miran a la estrella.
  - Difieren 6659 píxeles.
- **En vivo:** `--set clover.bump=0`.
- **Comando:** `cargo test --release --test p3_relief_normals_tests`

## 6. Cuatro cascadas

- **Qué es:** una cinta por montaña del trébol. Baja del borde de un lóbulo al mar por el gradiente del campo, con agua animada y espuma en la base.
- **Pruebas:** `tests/p3_four_waterfalls_tests.rs`, `tests/p3_waterfall_geometry_tests.rs` y `tests/p3_waterfall_shader_tests.rs`.
- **Captura:** [k3_waterfall.png](clover/k3_waterfall.png) (C53).
- **Comando:** `cargo test --release --test p3_four_waterfalls_tests --test p3_waterfall_geometry_tests --test p3_waterfall_shader_tests`

## 7. Cuatro tipos de modelo

- **Qué es:** árbol y roca en el trébol; edificio neón y pirámide escalonada en Hearts. Son cuatro geometrías procedurales propias. La ficha de cada una está en `surface_models.md`.
- **Prueba:** `tests/p3_surface_model_rubric_tests.rs`.
- **Capturas:** [k_close_flight.png](clover/k_close_flight.png) (árboles y rocas) y [m_close_flight.png](hearts/m_close_flight.png) (edificios y pirámide).
- **Comando:** `cargo test --release --test p3_surface_model_rubric_tests`

## 8. LOD y descarte, con triángulos

Los números son de `benchmark_final.md` (800×600, mediana de 3 corridas de 7 rondas):

| Caso | Con | Sin | Triángulos enviados |
|---|---|---|---|
| G (mirando al vacío) | descarte | descarte | 894 frente a 57 918 |
| K2 (trébol lejano) | descarte | descarte | 1746 frente a 41 514 |
| L1 (relieve lejano) | LOD automático | nivel 5 forzado | 2778 frente a 22 938 |
| L3 (relieve rasante) | LOD automático | nivel 5 forzado | 8608 frente a 44 128 |

- **Pruebas:** `tests/p3_culling_toggle_tests.rs` (mismo cuadro, menos triángulos), `tests/p3_culling_work_tests.rs`, `tests/p3_mesh_lod_tests.rs`, `tests/p3_lod_stats_tests.rs` y `tests/p3_lod_regression_tests.rs`.
- **En vivo:** `H` muestra los contadores del HUD, y `C` apaga y enciende el descarte. Su captura es `docs/screenshots/project3/06-hud-counters.png` (C78).
- **Comando:** `cargo test --release --test p3_culling_toggle_tests --test p3_culling_work_tests --test p3_mesh_lod_tests --test p3_lod_stats_tests --test p3_lod_regression_tests`
