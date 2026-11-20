# Proyecto 3 — Evidencia del planeta trébol (P3-05 y P3-06)

Clover Haven es el primer planeta con relieve propio. Tiene cuatro montañas con forma de trébol sobre un mar animado, árboles y rocas low-poly y cuatro cascadas. Todo sale de una semilla (`clover.seed`, 7 por defecto) y gira con el planeta.

## Commits

| Tarea | Commit | Qué dejó |
|---|---|---|
| C41 | `fb70b6c` | `CloverField`: cuatro tréboles (tres lóbulos, tallo y centro) en los vértices de un tetraedro girado por la semilla |
| C42 | `af58cc9` | Nivel del mar (0.02 R) y lecho marino con fbm; tierra solo dentro de los tréboles |
| C43 | `53afff8` | Material: pasto, musgo, roca y nieve por pendiente y altura; mar somero y profundo con Fresnel |
| C44 | `4d4d149` | Dos trenes de olas (periodo común de 60 s) y espuma a lo largo de la costa |
| C45 | `7f426da` | Dispersión de props sobre una red de Fibonacci con ruido, pendiente, tierra y espaciado |
| C46 | `caad14a` | Tres variantes de árbol low-poly (tronco hexagonal y dos o tres conos) |
| C47 | `4344bde` | Roca: icosfera de nivel 1 deformada por la semilla y aplastada |
| C48 | `f966cf9` | Props horneados en 12 sectores por parte (tronco, copa, roca) y descartados por sector |
| C49 | `eeb3243` | El trébol entra al sistema (órbita 21, R 1.45), con F3 de viaje rápido y los escenarios K y K2 |
| C50 | `51321cb` | Cinta de cascada: del borde de un lóbulo al mar por el gradiente del campo |
| C51 | `6fa6a1a` | Agua que baja por la cinta, con espuma en la base; se congela en pausa |
| C52 | `f32817c` | Una cascada por montaña, el escenario K3 y una prueba de horizonte que reemplaza el descarte por coseno |
| C53 | este commit | `tests/p3_clover_surface_contract_tests.rs`, las capturas y este documento |

## Capturas

Las tres son cuadros del benchmark escalado a 800×600, con t = 6 s y la semilla 7. Son los mismos cuadros cuyas huellas fija `P3_CLOVER_K` en `tests/p3_frame_oracle_tests.rs`.

| Escenario | Captura | Qué se ve |
|---|---|---|
| K (vuelo cercano) | [k_close_flight.png](clover/k_close_flight.png) | Lóbulos verdes, rocas, árboles, espuma en la costa y House of Cards al fondo |
| K2 (de lejos, 12 R) | [k2_from_afar.png](clover/k2_from_afar.png) | Un mundo oceánico con un trébol en el borde; nivel 2 y sin props (menos de 60 px) |
| K3 (cascada) | [k3_waterfall.png](clover/k3_waterfall.png) | Una montaña con su cascada en franjas, vista desde el mar a baja altura |

Para reproducirlas:

```sh
P3_DUMP_DIR=/tmp/clover cargo test --test p3_frame_oracle_tests p3_clover
# escribe /tmp/clover/p3_scaled_K.ppm, p3_scaled_K2.ppm y p3_scaled_K3.ppm
```

En la ventana: `cargo run --release -- --scene solar`, luego F3 (viaje rápido al trébol) y L (vuelo local).

## Contrato (`tests/p3_clover_surface_contract_tests.rs`)

| Prueba | Qué comprueba |
|---|---|
| `seed_controls_layout` | La misma semilla da las mismas poses. Con la semilla 8, más del 90 % de los props quedan donde la semilla 7 no tiene ninguno. |
| `props_follow_rotation` | En t = 0, 10 y 25 s, los props y las cascadas se dibujan con la matriz del planeta. |
| `no_props_under_sea` | Ningún árbol ni roca queda bajo el nivel del mar. |
| `four_waterfalls` | Hay cuatro cascadas y cada una empieza sobre tierra. |
| `two_model_kinds_present` | Hay árboles y rocas, y las tres partes horneadas (tronco, copa y roca) tienen props. |
| `lod_triangles_vary` | Con el planeta cerca (K), el nivel es mayor y hay más de cuatro veces los triángulos que de lejos (K2). |

**Otras suites del trébol:**
- `p3_clover_mountains_tests`, `p3_clover_sea_tests`, `p3_clover_material_tests` y `p3_clover_water_animation_tests`;
- `p3_scatter_seed_tests`, `p3_tree_mesh_tests`, `p3_rock_mesh_tests` y `p3_prop_sector_tests`;
- `p3_clover_orbit_tests`, `p3_waterfall_geometry_tests`, `p3_waterfall_shader_tests` y `p3_four_waterfalls_tests`.

## Costo

**Condiciones:** `--benchmark-solar --benchmark-scaled` en release, 800×600, 20 hilos, 1 calentamiento y 7 rondas. Tres corridas seguidas sobre `f32817c`, en el mismo binario. J (vuelo rasante sobre Crimson Entrance) es la referencia de la misma corrida.

| Caso | Corrida 1 | Corrida 2 | Corrida 3 | Mediana | Triángulos | Dibujos |
|---|---|---|---|---|---|---|
| J (referencia) | 75.27 | 77.92 | 74.85 | 75.27 | 2650 | 14 |
| K (cerca, nivel 4) | 69.05 | 43.69 | 62.63 | 62.63 | 17 890 | 38 |
| K2 (lejos, nivel 2) | 15.87 | 16.15 | 8.99 | 15.87 | 1746 | 7 |
| K3 (cascada) | 37.55 | 54.44 | 52.44 | 52.44 | 26 630 | 11 |
| D (sistema completo) | 25.02 | 23.90 | 24.23 | 24.23 | 3722 | 36 |

- **K y K3:** el peor caso del trébol (K) cuesta 0.83× J. El límite que obliga a revisar el LOD y el horneado es 2× J.
- **K2:** el trébol lejano cuesta como un planeta de esfera. El nivel 2 y los props ocultos lo mantienen barato.
- **Memoria:** el pico de RSS (`VmHWM`) queda entre 44 680 y 47 612 kB con todas las escenas del benchmark.
- **Ruido:** K varía entre 43.7 y 69.1 ms y K2 entre 9.0 y 16.2 ms, con los mismos contadores en las tres corridas. Es el ruido de la máquina (±25 %) ya registrado desde P3-01.

**Comparación con P3-04:** la línea base es la J sin relieve, 76.56 ms en `lod_baseline.md`. La J de hoy (75.27 ms) es la misma dentro del ruido: el trébol no encarece los cuadros donde no se ve.

## Hallazgo corregido en C52

K3 mostró un error de C48. El descarte por sector (coseno entre la normal del centro del sector y el ojo, menor que −0.3) quitaba sectores de rocas y copas cuyo borde cercano sí se veía en un vuelo bajo. Así, el cuadro con descarte no era igual al cuadro con `--no-culling`.

C52 lo reemplazó por una prueba de horizonte, `beyond_horizon` y `SolarScene::patch_hidden`. Un sector solo se omite cuando su punto más cercano queda detrás del horizonte de su punto más alto, con el planeta ocultando como su superficie más baja. `p3_four_waterfalls_tests::culled_when_behind` comprueba que, en K3, los dos cuadros son idénticos y que el descarte sí omite dibujos.

## Validación del cierre

- **G2 completo:** `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (todas las suites, `--no-fail-fast`) y `git diff --check`, todo en verde.
- **`cargo build --release`:** correcto.
- **Huellas:** `LEGACY_A_J` sin cambios. `P3_SCALED_A_J` se recapturó una vez en C49, con su causa en el código: el trébol entra al sistema. `P3_CLOVER_K` (K, K2 y K3) se capturó en C49 y C52.
