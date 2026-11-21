# Proyecto 3 — Matriz de puntos reclamados (200)

Esta tabla recoge los 17 rubros del menú oficial: el estado de cada uno, el commit que lo implementó, las suites que lo prueban, el comando para correrlas y la captura donde se ve.

- `tests/p3_rubric_evidence_tests.rs` lee esta tabla.
- Toda fila `VERIFICADO` debe apuntar a suites y capturas que existan en el repositorio.
- Una fila sin suites dice `NO IMPLEMENTADO`.
- Una fila `PARCIAL` está implementada y probada, pero le falta la captura. No reclama puntos seguros.

**Columnas de puntos:**
- **Seguro:** lo que hoy se puede defender en la demostración con prueba y captura.
- **Condicionado:** el máximo si se completa la evidencia pendiente **y** el profesor la acepta.

**Una prueba verde no es aceptación docente.** Un rubro suma solo si funciona en la demostración y el equipo sabe explicarlo. La nota es min(100, puntos aceptados), y ningún total de esta tabla está garantizado.

**Quinto planeta:** el autor aprobó el Joker el 2026-11-20 (`docs/project3_fifth_planet_decision.md`). El sistema tiene cinco planetas, sin contar la estrella, y el menú cuenta como máximo cinco: el techo condicionado es **200**.

| # | Rubro | Máx. | Estado | Cnn | Suites | Comando | Captura | Seguro | Condicionado |
|---|---|---:|---|---|---|---|---|---:|---:|
| 1 | Planetas, 10 por planeta (5 de 5; sin la estrella) | 50 | VERIFICADO | C23, C49, C61, Joker | `p3_clover_orbit_tests`, `p3_hearts_orbit_tests`, `p3_scale_ratios_tests`, `p3_joker_planet_tests` | `cargo test --release --test p3_clover_orbit_tests --test p3_hearts_orbit_tests --test p3_scale_ratios_tests --test p3_joker_planet_tests` | `docs/screenshots/lab6/02-crimson-entrance.png`, `docs/results/project3/clover/k_close_flight.png`, `docs/results/project3/hearts/m_close_flight.png`, `docs/screenshots/lab6/03-house-of-cards.png`, `docs/screenshots/project3/07-joker-close.png` | 50 | 50 |
| 2 | Tipos de modelo sobre la superficie, 5 por tipo | 20 | VERIFICADO | C46, C47, C57, C59, C62 | `p3_surface_model_rubric_tests`, `p3_tree_mesh_tests`, `p3_rock_mesh_tests`, `p3_neon_building_mesh_tests`, `p3_heart_pyramid_tests` | `cargo test --release --test p3_surface_model_rubric_tests --test p3_tree_mesh_tests --test p3_rock_mesh_tests --test p3_neon_building_mesh_tests --test p3_heart_pyramid_tests` | `docs/results/project3/clover/k_close_flight.png`, `docs/results/project3/hearts/m_close_flight.png` | 20 | 20 |
| 3 | Distribución procedural con semilla y ruido | 10 | VERIFICADO | C45, C58 | `p3_scatter_seed_tests`, `p3_hearts_city_distribution_tests` | `cargo test --release --test p3_scatter_seed_tests --test p3_hearts_city_distribution_tests` | `docs/results/project3/clover/k2_from_afar.png` | 10 | 10 |
| 4 | Posición y orientación sobre la superficie al girar | 5 | VERIFICADO | C48, C58, C62 | `p3_prop_sector_tests`, `p3_surface_model_rubric_tests` | `cargo test --release --test p3_prop_sector_tests --test p3_surface_model_rubric_tests` | `docs/results/project3/hearts/m_close_flight.png` | 5 | 5 |
| 5 | Shaders propios distinguibles, 6 por shader (4) | 24 | VERIFICADO | C43, C51, C55 | `lab6_final_contract_tests`, `p3_clover_material_tests`, `p3_hearts_grid_tests`, `p3_waterfall_shader_tests` | `cargo test --release --test lab6_final_contract_tests --test p3_clover_material_tests --test p3_hearts_grid_tests --test p3_waterfall_shader_tests` | `docs/screenshots/lab6/01-kings-crown.png`, `docs/screenshots/lab6/02-crimson-entrance.png`, `docs/screenshots/lab6/03-house-of-cards.png`, `docs/results/project3/clover/k3_waterfall.png` | 24 | 24 |
| 6 | Shader animado | 8 | VERIFICADO | C44, C51, C56 | `p3_clover_water_animation_tests`, `p3_hearts_glitch_tests` | `cargo test --release --test p3_clover_water_animation_tests --test p3_hearts_glitch_tests` | `docs/videos/lab6/02-crimson-entrance-events.gif` | 8 | 8 |
| 7 | Mapeo de normales (mapa de normales en textura con marco TBN; normal del campo y bump procedural) | 8 | VERIFICADO | C38, C44, `28c6e4c` | `p3_relief_normals_tests`, `p3_normal_map_tests` | `cargo test --release --test p3_relief_normals_tests --test p3_normal_map_tests` | `docs/screenshots/lab6/05-entrance-bump-off-on.png`, `docs/screenshots/project3/27-normal-map-off-on.png` | 8 | 8 |
| 8 | Deformación de la geometría con ruido | 10 | VERIFICADO | C35, C41, C42 | `p3_relief_geometry_tests`, `p3_clover_mountains_tests` | `cargo test --release --test p3_relief_geometry_tests --test p3_clover_mountains_tests` | `docs/results/project3/clover/k3_waterfall.png` | 10 | 10 |
| 9 | Generación o subdivisión procedural antes de rasterizar | 10 | VERIFICADO | C33, C76 | `p3_icosphere_topology_tests`, `p3_procedural_scene_tests` | `cargo test --release --test p3_icosphere_topology_tests --test p3_procedural_scene_tests` | `docs/results/project3/geometry/subdivision_l2_l5.png` | 10 | 10 |
| 10 | Vuelo local alrededor de un planeta | 8 | VERIFICADO | C32 | `solar_flight_tests`, `p3_local_terrain_collision_tests` | `cargo test --release --test solar_flight_tests --test p3_local_terrain_collision_tests` | `docs/screenshots/preproject3/03-local-flight.png` | 8 | 8 |
| 11 | Aceleración, frenado y giro progresivos | 8 | VERIFICADO | C25, C26, C27, C28, C77 | `p3_motion_timestep_tests`, `p3_ship_velocity_tests`, `p3_turn_easing_tests`, `p3_final_tour_tests` | `cargo test --release --test p3_motion_timestep_tests --test p3_ship_velocity_tests --test p3_turn_easing_tests --test p3_final_tour_tests` | `docs/videos/project3/01-final-tour.mp4` | 8 | 8 |
| 12 | Atracción gravitatoria | 10 | VERIFICADO | C29, C30, C74 | `p3_gravity_force_tests`, `p3_gravity_toggle_tests`, `p3_gravity_collision_integration_tests` | `cargo test --release --test p3_gravity_force_tests --test p3_gravity_toggle_tests --test p3_gravity_collision_integration_tests` | `docs/screenshots/project3/05-gravity-on-off.png` | 10 | 10 |
| 13 | Colisión con los planetas | 4 | VERIFICADO | C31, C32, C74, corrección del 2026-11-20 | `p3_collision_sweep_tests`, `p3_local_terrain_collision_tests`, `p3_gravity_collision_integration_tests`, `p3_collision_moving_target_tests` | `cargo test --release --test p3_collision_sweep_tests --test p3_local_terrain_collision_tests --test p3_gravity_collision_integration_tests --test p3_collision_moving_target_tests` | `docs/screenshots/project3/08-collision-10fps.png` | 4 | 4 |
| 14 | Descarte fuera de la vista | 7 | VERIFICADO | C06, C07, C08, C09, C10, C75 | `p3_culling_toggle_tests`, `p3_culling_work_tests`, `p3_benchmark_csv_tests` | `cargo test --release --test p3_culling_toggle_tests --test p3_culling_work_tests --test p3_benchmark_csv_tests` | `docs/screenshots/project3/06-hud-counters.png` | 7 | 7 |
| 15 | Niveles de detalle según la distancia | 8 | VERIFICADO | C37, C39, C40, C64, C75 | `p3_mesh_lod_tests`, `p3_lod_stats_tests`, `p3_lod_regression_tests`, `p3_neon_ship_lod_tests` | `cargo test --release --test p3_mesh_lod_tests --test p3_lod_stats_tests --test p3_lod_regression_tests --test p3_neon_ship_lod_tests` | `docs/screenshots/project3/06-hud-counters.png`, `docs/results/project3/geometry/subdivision_l2_l5.png` | 8 | 8 |
| 16 | Coherencia visual y composición (juicio docente) | 5 | VERIFICADO | C49, C61, C68 | `p3_frame_oracle_tests` | `cargo test --release --test p3_frame_oracle_tests` | `docs/screenshots/preproject3/01-system-orbit.png`, `docs/results/project3/clover/k_close_flight.png`, `docs/results/project3/hearts/m_close_flight.png` | 3 | 5 |
| 17 | Claridad de la experiencia (juicio docente) | 5 | VERIFICADO | C05, C07, C77, C78, C79 | `p3_hud_stats_tests`, `p3_delivery_contract_tests`, `p3_final_tour_tests` | `cargo test --release --test p3_hud_stats_tests --test p3_delivery_contract_tests --test p3_final_tour_tests` | `docs/screenshots/project3/06-hud-counters.png`, `docs/videos/project3/01-final-tour.mp4` | 3 | 5 |

## Notas por fila

- **1:** Crimson Entrance, Clover Haven, Neon Hearts, House of Cards y el Joker (expansión del 2026-11-20: relieve propio de rombos facetados y shader de arlequín, `p3_joker_planet_tests`). La estrella (The King's Crown) no cuenta como planeta.
- **5:** hay más implementaciones que las cuatro que puntúan: corona, Crimson Entrance, House of Cards, lazos de plasma, cartas, llamas, trébol, cascada, rejilla neón y neón de las naves. Cada una tiene una implementación propia, no solo otro color.
- **7:** antes de `28c6e4c` el sombreado usaba la normal del campo de altura y el bump procedural de Crimson Entrance, sin mapas de normales en textura, y solo 4 puntos eran seguros. Desde ese commit hay mapeo de normales clásico (`src/solar/planet/normal_map.rs`):
  - **Textura:** un mapa de normales RGB de 256×256 (`rgb = n · 0.5 + 0.5`), horneado una vez desde un mapa de alturas de roca que se repite sin costuras ([textura en mosaico de 2×2](results/project3/geometry/normal_map_rock.png)).
  - **Lectura:** filtro bilineal en coordenadas equirectangulares de la superficie, `u` hacia el este y `v` hacia el norte.
  - **TBN:** el texel pasa al espacio del objeto con `n = T·x + B·y + N·z` (T este, B norte, N la normal del relieve).
  - **Dónde:** ilumina la roca de las montañas de Clover Haven a escala Grand; `B` lo apaga y enciende en el visor y `--set clover.normal_map` cambia su intensidad (captura 27).
- **9, 11, 12, 14, 15, 17:** pasaron de `PARCIAL` a `VERIFICADO` en C79 con las capturas finales: la subdivisión L2/L5 (C76), el video del recorrido final (C77–C78), la gravedad con y sin `G` (C78) y el HUD con los contadores de descarte y el nivel de detalle por planeta (C78). Las cifras de descarte y LOD están en `docs/results/project3/benchmark_final.md` (C75).
- **13:** la colisión está implementada y probada: barrido, terreno local y turbo contra los planetas (C74). La corrección del 2026-11-20 hace que cada forma arranque donde estaba su planeta al inicio del cuadro. Con eso la nave conserva el margen de 0.05 con los cinco planetas a 10, 15, 30, 60 y 144 FPS (`p3_collision_moving_target_tests`; `docs/results/project3/gravity_evidence.md`). La captura 08 muestra a la nave detenida tras 5 s de turbo contra Clover a 10 FPS. Límites declarados:
  - en vuelo espacial, la forma es la esfera del relieve más alto; el vuelo local (`L`) sigue el terreno exacto;
  - las cartas del anillo de House of Cards no tienen colisión propia.
- **16–17:** son juicio docente: la tabla solo reclama una parte como segura.
- **Expansión del 2026-11-20:** no abre rubros nuevos. Lo que suma lo juzga el profesor en la demostración y no se reclama como puntos seguros:
  - captura orbital y escape tangencial (`p3_orbital_capture_tests`, capturas 09–10, GIF 02);
  - cielo animado, cometas, cascadas monumentales y oleaje;
  - luces automáticas y audio procedural.

  Refuerzan los rubros 6, 10, 16 y 17.

## Totales

- **Máximo del menú:** 200.
- **Seguro:** 196 (178 hasta el 2026-11-20, más 10 del Joker y 4 de la colisión con captura; 4 más del mapa de normales en textura, `28c6e4c`).
- **Condicionado:** 200.

Ninguno de estos totales está garantizado: el profesor acepta o no cada rubro en la demostración. La nota es min(100, puntos aceptados).
