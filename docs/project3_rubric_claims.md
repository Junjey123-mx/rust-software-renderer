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

**Quinto planeta:** `ART_PENDING_APPROVAL` (`docs/project3_fifth_planet_decision.md`). El sistema tiene cuatro planetas, sin contar la estrella. Con cuatro planetas, el techo condicionado es **190**.

| # | Rubro | Máx. | Estado | Cnn | Suites | Comando | Captura | Seguro | Condicionado |
|---|---|---:|---|---|---|---|---|---:|---:|
| 1 | Planetas, 10 por planeta (4 de 5; sin la estrella) | 50 | VERIFICADO | C23, C49, C61 | `p3_clover_orbit_tests`, `p3_hearts_orbit_tests`, `p3_scale_ratios_tests` | `cargo test --release --test p3_clover_orbit_tests --test p3_hearts_orbit_tests --test p3_scale_ratios_tests` | `docs/screenshots/lab6/02-crimson-entrance.png`, `docs/results/project3/clover/k_close_flight.png`, `docs/results/project3/hearts/m_close_flight.png`, `docs/screenshots/lab6/03-house-of-cards.png` | 40 | 40 |
| 2 | Tipos de modelo sobre la superficie, 5 por tipo | 20 | VERIFICADO | C46, C47, C57, C59, C62 | `p3_surface_model_rubric_tests`, `p3_tree_mesh_tests`, `p3_rock_mesh_tests`, `p3_neon_building_mesh_tests`, `p3_heart_pyramid_tests` | `cargo test --release --test p3_surface_model_rubric_tests --test p3_tree_mesh_tests --test p3_rock_mesh_tests --test p3_neon_building_mesh_tests --test p3_heart_pyramid_tests` | `docs/results/project3/clover/k_close_flight.png`, `docs/results/project3/hearts/m_close_flight.png` | 20 | 20 |
| 3 | Distribución procedural con semilla y ruido | 10 | VERIFICADO | C45, C58 | `p3_scatter_seed_tests`, `p3_hearts_city_distribution_tests` | `cargo test --release --test p3_scatter_seed_tests --test p3_hearts_city_distribution_tests` | `docs/results/project3/clover/k2_from_afar.png` | 10 | 10 |
| 4 | Posición y orientación sobre la superficie al girar | 5 | VERIFICADO | C48, C58, C62 | `p3_prop_sector_tests`, `p3_surface_model_rubric_tests` | `cargo test --release --test p3_prop_sector_tests --test p3_surface_model_rubric_tests` | `docs/results/project3/hearts/m_close_flight.png` | 5 | 5 |
| 5 | Shaders propios distinguibles, 6 por shader (4) | 24 | VERIFICADO | C43, C51, C55 | `lab6_final_contract_tests`, `p3_clover_material_tests`, `p3_hearts_grid_tests`, `p3_waterfall_shader_tests` | `cargo test --release --test lab6_final_contract_tests --test p3_clover_material_tests --test p3_hearts_grid_tests --test p3_waterfall_shader_tests` | `docs/screenshots/lab6/01-kings-crown.png`, `docs/screenshots/lab6/02-crimson-entrance.png`, `docs/screenshots/lab6/03-house-of-cards.png`, `docs/results/project3/clover/k3_waterfall.png` | 24 | 24 |
| 6 | Shader animado | 8 | VERIFICADO | C44, C51, C56 | `p3_clover_water_animation_tests`, `p3_hearts_glitch_tests` | `cargo test --release --test p3_clover_water_animation_tests --test p3_hearts_glitch_tests` | `docs/videos/lab6/02-crimson-entrance-events.gif` | 8 | 8 |
| 7 | Mapeo de normales (normal del campo y bump procedural) | 8 | VERIFICADO | C38, C44 | `p3_relief_normals_tests` | `cargo test --release --test p3_relief_normals_tests` | `docs/screenshots/lab6/05-entrance-bump-off-on.png` | 4 | 8 |
| 8 | Deformación de la geometría con ruido | 10 | VERIFICADO | C35, C41, C42 | `p3_relief_geometry_tests`, `p3_clover_mountains_tests` | `cargo test --release --test p3_relief_geometry_tests --test p3_clover_mountains_tests` | `docs/results/project3/clover/k3_waterfall.png` | 10 | 10 |
| 9 | Generación o subdivisión procedural antes de rasterizar | 10 | PARCIAL | C33 | `p3_icosphere_topology_tests` | `cargo test --release --test p3_icosphere_topology_tests` | pendiente (C76, C78) | 0 | 10 |
| 10 | Vuelo local alrededor de un planeta | 8 | VERIFICADO | C32 | `solar_flight_tests`, `p3_local_terrain_collision_tests` | `cargo test --release --test solar_flight_tests --test p3_local_terrain_collision_tests` | `docs/screenshots/preproject3/03-local-flight.png` | 8 | 8 |
| 11 | Aceleración, frenado y giro progresivos | 8 | PARCIAL | C25, C26, C27, C28 | `p3_motion_timestep_tests`, `p3_ship_velocity_tests`, `p3_turn_easing_tests` | `cargo test --release --test p3_motion_timestep_tests --test p3_ship_velocity_tests --test p3_turn_easing_tests` | pendiente (C77, C78) | 0 | 8 |
| 12 | Atracción gravitatoria | 10 | PARCIAL | C29, C30 | `p3_gravity_force_tests`, `p3_gravity_toggle_tests` | `cargo test --release --test p3_gravity_force_tests --test p3_gravity_toggle_tests` | pendiente (C74, C78) | 0 | 10 |
| 13 | Colisión con los planetas | 4 | PARCIAL | C31, C32 | `p3_collision_sweep_tests`, `p3_local_terrain_collision_tests` | `cargo test --release --test p3_collision_sweep_tests --test p3_local_terrain_collision_tests` | pendiente (C74) | 0 | 4 |
| 14 | Descarte fuera de la vista | 7 | PARCIAL | C06, C07, C08, C09, C10 | `p3_culling_toggle_tests`, `p3_culling_work_tests` | `cargo test --release --test p3_culling_toggle_tests --test p3_culling_work_tests` | pendiente (C78) | 0 | 7 |
| 15 | Niveles de detalle según la distancia | 8 | PARCIAL | C37, C39, C40, C64 | `p3_mesh_lod_tests`, `p3_lod_stats_tests`, `p3_lod_regression_tests`, `p3_neon_ship_lod_tests` | `cargo test --release --test p3_mesh_lod_tests --test p3_lod_stats_tests --test p3_lod_regression_tests --test p3_neon_ship_lod_tests` | pendiente (C78) | 0 | 8 |
| 16 | Coherencia visual y composición (juicio docente) | 5 | VERIFICADO | C49, C61, C68 | `p3_frame_oracle_tests` | `cargo test --release --test p3_frame_oracle_tests` | `docs/screenshots/preproject3/01-system-orbit.png`, `docs/results/project3/clover/k_close_flight.png`, `docs/results/project3/hearts/m_close_flight.png` | 3 | 5 |
| 17 | Claridad de la experiencia (juicio docente) | 5 | PARCIAL | C05, C07 | `p3_hud_stats_tests`, `p3_delivery_contract_tests` | `cargo test --release --test p3_hud_stats_tests --test p3_delivery_contract_tests` | pendiente (C78, C79) | 0 | 5 |

## Notas por fila

- **1:** Crimson Entrance, Clover Haven, Neon Hearts y House of Cards. La estrella (The King's Crown) no cuenta como planeta. El quinto planeta no está aprobado, así que la fila reclama 40 de 50.
- **5:** hay más implementaciones que las cuatro que puntúan: corona, Crimson Entrance, House of Cards, lazos de plasma, cartas, llamas, trébol, cascada, rejilla neón y neón de las naves. Cada una tiene una implementación propia, no solo otro color.
- **7:** el sombreado usa la normal del campo de altura y el bump procedural de Crimson Entrance. No hay mapas de normales en textura, y por eso solo 4 puntos son seguros.
- **9, 11–15, 17:** el código y las pruebas existen. La captura llega con las evidencias finales (C74, C76–C79), y hasta entonces la fila no reclama puntos seguros.
- **16–17:** son juicio docente: la tabla solo reclama una parte como segura.
