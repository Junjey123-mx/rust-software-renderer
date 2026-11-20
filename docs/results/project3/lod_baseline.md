# Proyecto 3 — Cierre de P3-04 (geometría de relieve y niveles de detalle)

P3-04 añadió la geometría de relieve: icosferas indexadas, un campo de alturas común, mallas desplazadas, caché de geometría, selección de nivel de detalle (LOD) por tamaño en pantalla, normales del campo en el sombreado y contadores por planeta. Ningún cuerpo del sistema de la entrega tiene relieve todavía: el primero será el planeta del trébol (P3-05). Por eso estas medidas usan un planeta de prueba con `TestField`, que solo existe en el benchmark.

## Commits

| Tarea | Commit | Qué dejó |
|---|---|---|
| C33 | `c83d311` | Icosfera indexada, niveles 0–5, sin vértices repetidos |
| C34 | `0886c28` | `HeightField` (`Flat`, `Test`) con altura, nivel del mar, radio de superficie y normal |
| C35 | `1a94ea9` | `relief_mesh`: la icosfera desplazada por el campo, con el mar dentro de la superficie |
| C36 | `c7dbcec` | `GeometryCache`: una sola copia de los niveles 2–5 por radio y campo, construida al arrancar |
| C37 | `515da1f` | `lod_for` (sin historia: grabación y benchmark) y `lod_step` (con histéresis: la ventana) |
| C38 | `50e810b` | Sombreado con la normal del campo (`NORMAL_EPS` = 1e-3), con `bump` 0 idéntico al de la esfera |
| C39 | `f3a406c` | Nivel y triángulos por cuerpo en `FrameStats`, una línea por planeta en el HUD y la columna `lod_levels` del CSV |
| C40 | este commit | Escenarios L1–L3 del benchmark, `lod_force`, `tests/p3_lod_regression_tests.rs` y este documento |

## Umbrales del LOD

| Nivel | Triángulos | Sube desde | Baja por debajo de |
|---|---|---|---|
| 2 | 320 | — | — |
| 3 | 1280 | 48 px | 40 px |
| 4 | 5120 | 140 px | 120 px |
| 5 | 20480 | 360 px | 300 px |

Los píxeles son el radio proyectado del cuerpo. `lod_for` sube desde el nivel 2 sin historia: entre 40 y 48 px da el nivel 2.

## Escenarios L1–L3

Los escenarios L1–L3 usan Crimson Entrance con `TestField` (amplitud 0.04 R, frecuencia 6, mar en 0). Solo existen en el sistema escalado (`--benchmark-solar --benchmark-scaled`). Cada uno corre con el LOD automático y, justo después, con el nivel 5 forzado (`L1F`, `L2F` y `L3F`).

| Caso | Cámara | Nivel automático a 800×600 |
|---|---|---|
| L1 | Nave a 15 R del centro, de frente | 2 |
| L2 | Nave a 2 R del centro, de frente | 4 |
| L3 | Vuelo rasante de J (holgura del planeta) | 4 |

**Sobre L3:** el vuelo rasante ve unos 283 px de radio a 800×600, por debajo de los 360 px del nivel 5. El nivel 5 llega con cuadros de unos 770 px de alto o más (por ejemplo, 1280×800 o 1920×1080).

## Medidas

**Condiciones:** `--benchmark-solar --benchmark-scaled` en release, 800×600, 20 hilos, 1 calentamiento y 7 rondas. Tres corridas seguidas sobre `f3a406c` más los cambios de C40, en el mismo binario.

### Mediana en ms

| Caso | Corrida 1 | Corrida 2 | Corrida 3 | Mediana de las tres | Triángulos enviados |
|---|---|---|---|---|---|
| J (referencia, sin relieve) | 76.60 | 76.56 | 73.58 | 76.56 | 2650 |
| L1 automático (nivel 2) | 19.11 | 31.55 | 34.38 | 31.55 | 2778 |
| L1F (nivel 5) | 40.01 | 41.65 | 38.83 | 40.01 | 22938 |
| L2 automático (nivel 4) | 67.19 | 66.45 | 69.28 | 67.19 | 6014 |
| L2F (nivel 5) | 73.39 | 74.29 | 73.31 | 73.39 | 21374 |
| L3 automático (nivel 4) | 84.99 | 83.87 | 85.44 | 84.99 | 7578 |
| L3F (nivel 5) | 92.44 | 86.14 | 88.25 | 88.25 | 22938 |

### Costo de los triángulos extra del nivel 5 forzado

La tabla divide la diferencia de medianas entre los triángulos de más.

| Caso | Triángulos extra | ms extra | µs por triángulo extra |
|---|---|---|---|
| L1 | 20160 | 8.46 | 0.42 |
| L2 | 15360 | 6.21 | 0.40 |
| L3 | 15360 | 3.26 | 0.21 |

### Memoria

El pico de RSS del proceso (`VmHWM`) queda entre 31 752 y 42 268 kB en las tres corridas, con todas las escenas del benchmark. La caché mide 2 937 600 B por radio y campo (niveles 2–5), según `GeometryCache::bytes`: unos 2.9 MB compartidos frente a 11.75 MB sin caché para cuatro planetas iguales.

## Decisión del tope

**Se mantiene el nivel 5.**

- El nivel 5 forzado en el peor caso (L3F) cuesta 88.25 ms, 1.15× la J de la misma corrida (76.56 ms), muy por debajo del límite de 2× J que obligaría a bajar el tope a 4.
- El LOD automático sí ahorra trabajo: a distancia (L1), el planeta envía 2778 triángulos en lugar de 22938 y el cuadro baja de unos 40 ms a 19–34 ms. El costo por triángulo extra es de 0.2–0.4 µs.
- Cerca de la superficie el costo lo dominan los fragmentos, no los triángulos (L3 frente a L3F: +4 %).

**Ruido:** L1 varía entre 19.1 y 34.4 ms sin cambiar de imagen ni de contadores. Es el mismo ruido de la máquina (±25 %) ya registrado en P3-01. Los contadores (`triangles_submitted`, `triangles_culled` y `lod_levels`) son idénticos en las tres corridas.

## Validación del cierre

- **G2 completo:** `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (todas las suites, `--no-fail-fast`) y `git diff --check`, todo en verde.
- **`cargo build --release`:** correcto.
- **`tests/p3_lod_regression_tests.rs`:**
  - cada nivel forzado da la misma huella en dos escenas nuevas y sus triángulos son 20·4^nivel;
  - el LOD automático nunca envía más triángulos que el nivel 5 forzado;
  - L1–L3 usan los niveles 2, 4 y 4 a 800×600.
- **A–J sin cambios:** las filas, el CSV y las huellas `LEGACY_A_J` y `P3_SCALED_A_J` no cambian. Las filas L solo se añaden con `--benchmark-scaled`.
