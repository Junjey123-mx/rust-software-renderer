# Proyecto 3 — Cierre de P3-01 (refactorización a specs y adornos)

El checkpoint P3-01 movió el sistema solar a datos validados (`SystemSpec`), separó la identidad de material del enum `celestial::Body` del Laboratorio 6 y llevó la corona y el anillo de cartas a un contrato común de adornos. Ningún píxel cambió.

## Commits

| Tarea | Commit | Qué dejó |
|---|---|---|
| C09 | `4529f42` | `C` y `--no-culling`: el mismo cuadro con y sin descarte; el descarte vive en un solo punto (`sphere_in_view`) |
| C10 | `5c6efba` | Casos G2 (planeta cortado por el borde) y G3 (cámara girada 90°) fuera de A–J; ningún falso positivo |
| C11 | `b1213dd` | `PlanetMaterial::Lab6` y `PreparedPlanet`: delegan en `prepare_instance` y `shade_instance`, con bits idénticos |
| C12 | `686b4c2` | `SystemSpec`, `PlanetSpec`, `StarSpec` y `validate` con errores propios; `preproject3()` reproduce los valores cableados |
| C13 | `0333aaf` | `SolarScene::from_spec`; `default_system` construye la escena desde `SystemSpec::preproject3()` |
| C14 | `f45ef61` | Contrato de adornos: anfitrión por id, duplicados y anfitriones inexistentes con error limpio; `extent` medido |
| C15 | `bc73c5a` | `effect_draws` recorre `SolarScene::attachments` en orden de dibujo (corona y anillo) |
| C16 | este commit | `tests/p3_refactor_regression_tests.rs` y este documento |

## Huellas

- **Escenarios A–J:** las huellas de `tests/p3_frame_oracle_tests.rs` (`LEGACY_A_J`, congeladas en `ea3b4a8`) siguen iguales después de cada commit de P3-01.
- **Comprobación adicional:**
  - `tests/p3_scene_spec_oracle_tests.rs` compara la escena del spec con una copia de la construcción cableada.
  - `tests/p3_attachment_oracle_tests.rs` vuelve a comprobar A–J con los adornos.
- **Fuentes del Laboratorio 6:** `src/celestial/` no cambió, salvo los flags aditivos de `cli.rs`. `celestial_sources_untouched` guarda el hash de los otros 11 archivos. Al añadir un solo byte a `house.rs` en una copia local, la prueba falló, como se esperaba.

## Comportamiento congelado (`tests/p3_refactor_regression_tests.rs`)

| Prueba | Qué fija |
|---|---|
| `orbits_unchanged` | Posición y giro de los tres cuerpos en t = 0, 3.7 y 60 s, con las fórmulas de `ea3b4a8` |
| `pause_semantics_unchanged` | En pausa, el tiempo simulado y las órbitas se detienen, `real_delta` sigue y la nave vuela |
| `warp_unchanged` | Llegada de F1–F3 en t = 6 s, nariz hacia el cuerpo y salida del vuelo local (`travel.rs` no cambió desde `ea3b4a8`) |
| `rigs_and_local_flight_unchanged` | `V` alterna Follow y Orbit; `L` entra y sale del vuelo local y se rechaza lejos de los planetas |
| `celestial_sources_untouched` | Hash de `src/celestial/` sin `cli.rs`, con fin de línea normalizado |

## Validación del cierre

- **G2 completo:** `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (todas las suites, `--no-fail-fast`) y `git diff --check`, todo en verde.
- **`cargo build --release`:** correcto.
- **Benchmark:** `--benchmark-solar` en 800×600 con 20 hilos, tres corridas sobre `bc73c5a`. Mediana en ms:

| Escenario | Corrida 1 | Corrida 2 | Corrida 3 | Línea base C02 |
|---|---|---|---|---|
| A | 8.08 | 9.35 | 8.77 | 9.12 |
| D | 18.95 | 17.71 | 17.45 | 17.14 |
| E | 34.67 | 32.50 | 20.96 | 21.25 |
| F | 14.71 | 26.11 | 14.83 | 22.10 |
| G | 8.33 | 9.77 | 9.25 | 8.21 |
| J | 41.16 | 41.68 | 39.93 | 40.25 |

E y F varían entre corridas (20.9–34.7 ms y 14.7–26.1 ms) sin que cambie el código ni la imagen: es ruido de la máquina, del mismo orden que el ±25 % ya observado en el preproyecto. Los contadores (`draws`, `triangles_submitted`) son idénticos en las tres corridas.

## Desvíos registrados

- **Corrección del commit C08:** `dbf3a10` se registró con una prueba unitaria en rojo. La corrigió `3c46920`, un commit fuera del catálogo, sin `amend`. Desde entonces, G2 corre `cargo test --no-fail-fast`, y un commit solo se hace con G2 en verde.
- **G2/G3:** quedaron fuera de `SCENARIOS` para no alterar A–J, su CSV ni sus huellas.
- **Escala en lugar de radio:** el spec guarda la escala de la malla, porque 1.3 × 0.866 / 0.866 ≠ 1.3 en f32.
- **Corona:** su alcance medido es 1.7615 R (el documento B decía 1.751 R) y el documento se corrigió.
