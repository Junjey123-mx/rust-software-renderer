# Proyecto 3 — Validación final (C01–C80)

Cierre del Gate maestro del Proyecto 3 (opción B, ruta de cuatro planetas). Este documento registra:

- el estado real de cada commit;
- las verificaciones de calidad (G2) y de identidad (G4);
- lo que queda pendiente.

La auditoría detallada vive fuera del repositorio.

**Una prueba verde no es aceptación docente.** Ningún total de puntos está garantizado.

## Identificación

- **Rama:** `main`, sin push. La integración y el push quedan a decisión del usuario.
- **Base del Gate:** `ea3b4a8` (`origin/main`).
- **Commits del Gate:** 79 sobre la base:
  - 77 del catálogo antes de este (C01–C70 y C73–C79);
  - 1 correctivo fuera del catálogo (C08b);
  - este commit (C80).
- **No ejecutados:** C71 y C72.
- **Quinto planeta:** `ART_PENDING_APPROVAL` ([decisión](project3_fifth_planet_decision.md)). El sistema tiene cuatro planetas más la estrella, y el techo condicionado es 190.

## Estados

- **PASS:** el commit cumple su alcance con G2 en verde al confirmarlo.
- **PARTIAL:** se confirmó con algo pendiente o corregido después (ver nota).
- **NO EJECUTADO:** sin commit.

Los asuntos son los del catálogo, literales.

| Cnn | Hash | Asunto | Estado | Nota |
|---|---|---|---|---|
| C01 | `0f26c06` | test: freeze project three solar frame baselines | PASS |  |
| C02 | `fb7ba39` | perf: establish project three release benchmarks | PASS |  |
| C03 | `b4b59ba` | test: document all mandatory project delivery gates | PASS |  |
| C04 | `fb2da58` | feat: script a continuous demonstration flight | PASS |  |
| C05 | `dd75e46` | docs: add a provisional project three delivery guide | PASS |  |
| C06 | `5d2e8af` | feat: accumulate solar frame rendering counters | PASS |  |
| C07 | `00117a5` | feat: display render counters in the solar HUD | PASS |  |
| C08 | `dbf3a10` | perf: export triangle counters to benchmark reports | PARTIAL | Se confirmó con una prueba unitaria en rojo (`csv_header_keeps_the_old_columns_first`). No se enmendó; lo corrige C08b. |
| C08b | `3c46920` | test: align the solar csv header unit test with version two | PASS | Commit correctivo fuera del catálogo: alinea la prueba con la cabecera CSV v2. |
| C09 | `4529f42` | feat: toggle solar frustum culling for comparison | PASS |  |
| C10 | `5c6efba` | test: prove culling savings using offscreen planets | PASS |  |
| C11 | `b1213dd` | refactor: define independent solar planet material types | PASS |  |
| C12 | `686b4c2` | refactor: add structured system and planet specifications | PASS |  |
| C13 | `0333aaf` | refactor: construct existing solar bodies from system specs | PASS |  |
| C14 | `f45ef61` | refactor: define host-bound solar attachments | PASS |  |
| C15 | `bc73c5a` | refactor: migrate crown and card ring into attachments | PASS |  |
| C16 | `9f28a88` | test: lock spec and attachment refactor behavior | PASS |  |
| C17 | `d8a5f2a` | feat: generalize attachment ownership by planet id | PASS |  |
| C18 | `3e8e562` | feat: support validated live solar parameters | PASS |  |
| C19 | `a06c862` | feat: centralize solar relative scales | PASS |  |
| C20 | `c74a76a` | feat: calculate effective extents for celestial effects | PASS |  |
| C21 | `70b4a02` | feat: solve monotonically increasing orbital radii | PASS |  |
| C22 | `e2ece1b` | refactor: derive travel and camera distances from scale | PASS |  |
| C23 | `4b3105c` | feat: rescale the complete solar system composition | PASS |  |
| C24 | `c4ce4b8` | feat: constrain orbital angular speed by linear reachability | PASS |  |
| C25 | `caf8d2e` | feat: store inertial spacecraft linear velocity | PASS |  |
| C26 | `0ae85c0` | feat: smooth spacecraft yaw and pitch with delta time | PASS |  |
| C27 | `3dfca6a` | feat: integrate boosted thrust with progressive braking | PASS |  |
| C28 | `28cb0f9` | feat: advance ship motion with stable integration substeps | PASS |  |
| C29 | `e9a3c13` | feat: calculate planetary gravity on the ship | PASS |  |
| C30 | `0d06d3d` | feat: expose a reversible gravity comparison mode | PASS |  |
| C31 | `dfef77c` | feat: stop spacecraft penetration into spherical bodies | PASS |  |
| C32 | `279ac1a` | feat: integrate terrain-aware local flight and collision | PASS |  |
| C33 | `c83d311` | feat: generate indexed subdivided icosphere meshes | PASS |  |
| C34 | `0886c28` | feat: define a shared procedural height field interface | PASS |  |
| C35 | `1a94ea9` | feat: deform planet vertices from height fields | PASS |  |
| C36 | `c7dbcec` | feat: cache planet geometry across levels of detail | PASS |  |
| C37 | `515da1f` | feat: select mesh level of detail from projected size | PASS |  |
| C38 | `50e810b` | feat: shade surface relief using height field normals | PASS |  |
| C39 | `f3a406c` | feat: report per-planet LOD and geometry workload | PASS |  |
| C40 | `f8e7718` | perf: benchmark geometric LOD and relief extremes | PASS |  |
| C41 | `fb70b6c` | feat: define clover-shaped mountain distance fields | PASS |  |
| C42 | `af58cc9` | feat: carve a consistent sea level into clover relief | PASS |  |
| C43 | `53afff8` | feat: shade clover vegetation and ocean materials | PASS |  |
| C44 | `4d4d149` | feat: animate clover ocean waves and coastal foam | PASS |  |
| C45 | `7f426da` | feat: scatter clover surface props with seeded noise | PASS |  |
| C46 | `caad14a` | feat: generate reusable low-poly clover trees | PASS |  |
| C47 | `4344bde` | feat: generate reusable rocky surface props | PASS |  |
| C48 | `f966cf9` | perf: bake clover props into culled sector meshes | PASS |  |
| C49 | `eeb3243` | feat: register the complete clover planet in the solar scene | PASS |  |
| C50 | `51321cb` | feat: generate waterfall ribbons along clover relief | PASS |  |
| C51 | `6fa6a1a` | feat: animate water flow across waterfall ribbons | PASS |  |
| C52 | `f32817c` | feat: distribute four clover waterfalls across the planet | PASS |  |
| C53 | `19dfb86` | test: verify clover surface assets and close navigation | PASS |  |
| C54 | `a232803` | feat: define futuristic hearts terrace terrain | PASS |  |
| C55 | `6488467` | feat: shade hearts with animated neon grid lines | PASS |  |
| C56 | `b4979bb` | feat: add controlled deterministic hearts glitch events | PASS |  |
| C57 | `27b43f1` | feat: generate low-poly neon towers and civic blocks | PASS |  |
| C58 | `b4a7bca` | perf: bake seeded hearts city districts by sector | PASS |  |
| C59 | `fc0cff8` | feat: generate the monumental hearts pyramid structure | PASS |  |
| C60 | `3ee1cc5` | feat: model the elevated heart identity disc | PASS |  |
| C61 | `7ad0eef` | feat: register the neon hearts planet and landmarks | PASS |  |
| C62 | `ceb97fc` | test: defend four unique surface model categories | PASS |  |
| C63 | `3cb47bd` | feat: define reusable deterministic local vehicle paths | PASS |  |
| C64 | `3996ff0` | feat: share neon spacecraft meshes with distance LOD | PASS |  |
| C65 | `5b7052e` | feat: animate neon ship traffic around hearts | PASS |  |
| C66 | `aebd431` | feat: add an original low-poly lightcycle asset | PASS | No llegó un OBJ del usuario: la moto de luz es un modelo procedural propio (alternativa del catálogo). |
| C67 | `d5560a7` | feat: animate lightcycles on curved city surface routes | PASS |  |
| C68 | `2c4373d` | feat: generate fixed-length emissive vehicle trails | PASS |  |
| C69 | `bcd2585` | perf: batch and cull neon traffic light trails | PASS |  |
| C70 | `7caac99` | docs: propose and approve a distinct fifth planet identity | PASS | Solo documenta los dos conceptos. El asunto del catálogo dice «approve», pero no hay aprobación: `FIFTH_PLANET = ArtPendingApproval`. |
| C71 | — | — | NO EJECUTADO | NO EJECUTADO POR APROBACIÓN ARTÍSTICA PENDIENTE. |
| C72 | — | — | NO EJECUTADO | NO EJECUTADO POR APROBACIÓN ARTÍSTICA PENDIENTE. |
| C73 | `6ee3161` | test: audit every official rubric category with proof | PASS |  |
| C74 | `a3d4c3f` | test: validate gravitational navigation and planetary collision | PASS | Límite documentado: a 10 FPS la forma de colisión adelanta al planeta hasta un cuadro (−0.035 u). |
| C75 | `7312298` | perf: benchmark the complete five-planet scene | PASS | El asunto del catálogo dice «five-planet»; el sistema medido tiene cuatro planetas. |
| C76 | `73c84f5` | test: prove procedural generation and model placement | PASS |  |
| C77 | `8b00663` | feat: record a continuous final tour through the solar system | PASS |  |
| C78 | `c6baa01` | docs: encode verified project video screenshots and gifs | PASS |  |
| C79 | `b8a3437` | docs: finalize project three README and architecture guide | PARTIAL | Las contribuciones del integrante siguen `PENDIENTE` (no se inventan); la declaración de IA está. |
| C80 | (este commit) | docs: finalize project three release audit and git checks | PASS | Este commit; su hash está en `git log` y en la auditoría externa. |

**Resumen:** 77 PASS, 2 PARTIAL (C08 y C79), 2 NO EJECUTADO (C71 y C72) y ningún BLOCKED.

## G2 — calidad (árbol de C80)

| Paso | Resultado |
|---|---|
| `cargo fmt --check` | OK, sin diferencias |
| `cargo check --all-targets` | OK, sin avisos ni errores |
| `cargo clippy --all-targets -- -D warnings` | OK, 0 avisos |
| `cargo test --no-fail-fast` | 987 pasadas, 0 fallidas |
| `git diff --check` | OK |
| `cargo build --release` | OK (`Finished release`) |

G2 se ejecutó el 2026-11-20 con `CARGO_TARGET_DIR` en `/tmp`, porque `/home` está al 100 %, y con `CARGO_INCREMENTAL=0` y el perfil de pruebas sin información de depuración. La semántica de las pruebas no cambia.

## G4 — identidad e higiene de git

- Todos los commits sobre `ea3b4a8` tienen como autor y committer la identidad git ya configurada del usuario.
- Ninguno lleva trailers de coautoría ni de generación (`Co-authored-by`, `Generated-by`, `Signed-off-by`).
- No se cambió `git config` y no se usó `--author`.
- No hubo push, merge, rebase, amend ni reset.
- Cada commit se preparó por rutas explícitas. `examples/` (sin seguimiento, del usuario) nunca se preparó.
- **Auditorías fuera del repositorio:** ningún archivo de auditoría vive en el repositorio. `p3_final_contract_tests::no_audit_files_in_repo` lo comprueba.

## Oráculos y modelos

- **Archivos fijados:** `p3_final_contract_tests::legacy_oracles_untouched` fija byte por byte, con los valores de `ea3b4a8`:
  - los oráculos de los laboratorios 4, 5 y 6;
  - el oráculo del preproyecto;
  - `assets/models/sphere.obj` y `assets/models/Laboratorio3.obj`.
- **`LEGACY_A_J`:** la misma prueba fija la tabla tal como quedó en C01.
- **Tablas de escena:** `P3_SCALED_A_J`, `P3_CLOVER_K` y `P3_HEARTS_M` se recapturaron solo en los commits que añadieron cuerpos o adjuntos, con la causa anotada (C49, C61, C65, C67, C68). No cambian desde C68.
- **Única prueba previa cambiada:** `tests/solar_benchmark_tests.rs`, por el número de columnas del CSV v2 (C02 y C08, documentado).

## Evidencia de entrega

- **Video:** `docs/videos/project3/01-final-tour.mp4`. H.264, 800×600, 30 fps, 56 s, 4.7 MB, sin warp ni cambio de cámara.
- **GIF:** `docs/videos/project3/01-final-tour.gif`. 320×240, 6 fps, 4.8 MB.
- **Capturas del framebuffer de CPU:** `docs/screenshots/project3/01`–`06`.
- **Matriz de puntos:** [project3_rubric_claims.md](project3_rubric_claims.md). Máximo 200, seguro 178 y condicionado 190; no garantizados.
- **Contrato R1–R5:** [project3_delivery_contract.md](project3_delivery_contract.md). R1–R4 `CUMPLE`; R5 `CUMPLE (provisional)` hasta que el integrante confirme sus contribuciones.

## Pendientes y límites

1. **Contribuciones:** el README las marca `PENDIENTE`; las confirma el integrante.
2. **Quinto planeta:** C71 y C72 esperan la aprobación artística del usuario.
3. **Colisión con planetas (rubro 13):** sin captura propia; se demuestra en vivo.
4. **Colisión a FPS bajos:** a 10 FPS la forma de colisión adelanta al planeta hasta un cuadro de su órbita (`docs/results/project3/gravity_evidence.md`).
5. **Push:** no se hizo; lo hace el usuario tras revisar.

## Expansión del 2026-11-20 (después de C80)

Esta sección se añade sin reescribir lo anterior. Todo lo que hay más arriba es el estado de C80 (`3bc7ad3`) tal como se cerró.

### Commits de la expansión

| Hash | Asunto | G2 al confirmar |
|---|---|---|
| `35baf3b` | docs: declare individual authorship and close the R5 contract | verde |
| `3b4fef0` | feat: add the approved Joker as the fifth Project 3 planet | verde |
| `da874ba` | fix: start collision shapes where moving planets were at frame start | verde |
| `e81786c` | docs: add collision evidence capture and verify the collision row | verde |
| `08501d2` | feat: cycle the Project 3 sky slowly from purple to blue and back | verde |
| `e36c659` | feat: add comets that travel through the Project 3 sky | verde, 1009 pasadas |
| `435c2b6` | feat: add orbital capture with hysteresis and tangential escape | verde, 1020 pasadas |
| `e09d9e3` | feat: add three monumental waterfalls and an ocean swell to the clover planet | verde, 1024 pasadas |
| `b7ec83e` | feat: switch the Project 3 ship lights on automatically in planet shadows | verde, 1028 pasadas |
| `cb64845` | feat: add procedural ship audio with an M toggle to the Project 3 viewer | verde, 1033 pasadas |

Los commits de documentación y capturas que cierran la expansión vienen después; sus hashes están en `git log` y en la auditoría externa. «Verde» significa `fmt`, `clippy -D warnings`, `cargo test --no-fail-fast` sin fallos y `git diff --check`.

### Estado actualizado

Los puntos pendientes de C80 quedan así:

- **R5:** `CUMPLE`. El autor confirmó que el proyecto es individual ([contrato](project3_delivery_contract.md)).
- **Quinto planeta:** el Joker está aprobado y activo. Ace Crystal está aprobado pero no implementado ([decisión](project3_fifth_planet_decision.md)).
- **Colisión (rubro 13):** `VERIFICADO` con la captura 08.
- **Colisión a FPS bajos:** corregida en `da874ba`. Conserva el margen de 0.05 u a 10, 15, 30, 60 y 144 FPS.
- **Puntos:** seguro 192 y condicionado 200, sobre el máximo de 200 ([matriz](project3_rubric_claims.md)). No están garantizados.
- **Oráculos:**
  - `LEGACY_A_J`, los oráculos de los laboratorios y los OBJ no cambiaron (`p3_final_contract_tests`);
  - `P3_SCALED_A_J`, `P3_CLOVER_K` y `P3_HEARTS_M` se recapturaron solo en los commits que cambian lo que se ve: el Joker, el cielo, los cometas, las cascadas con el oleaje y las luces;
  - la causa de cada recaptura está anotada en `tests/p3_frame_oracle_tests.rs`.
- **Pruebas previas cambiadas:**
  - las que contaban cuatro planetas o cuatro fuentes de gravedad, por el Joker (`3b4fef0`);
  - `boost_into_planet_no_penetration`, que ahora exige el margen (`da874ba`);
  - `p3_four_waterfalls_tests::culled_when_behind`, que cuenta 4 + 3 cascadas (`e09d9e3`).
- **Nuevas suites:**
  - `p3_joker_planet_tests`, `p3_collision_moving_target_tests`, `p3_sky_cycle_tests` y `p3_comet_tests`;
  - `p3_orbital_capture_tests`, `p3_monumental_waterfall_tests`, `p3_ship_lights_tests` y `p3_audio_tests`.
- **Capturas y GIF:** las capturas 09–15 y los GIF 02–11 están descritos en [media_expansion.md](results/project3/media_expansion.md).
- **Rendimiento:** [benchmark_expansion.md](results/project3/benchmark_expansion.md) mide `3bc7ad3` frente a `4f3b3af` y el costo de cada función. No hay regresión fuera del ruido.

### No ejecutado en la expansión

- **Ace Crystal:** un sexto cuerpo movería las órbitas, el punto de aparición y todos los oráculos a horas de la entrega, y no suma puntos porque el menú cuenta hasta cinco planetas.
- **Reescalado mayor:** se evaluó y no se aplicó ([evaluación](project3_rescale_evaluation.md)).
- **Video final:** lo graba el autor. El README deja el marcador «VIDEO FINAL — PENDIENTE DE INCORPORACIÓN POR EL AUTOR».
- **Push:** no se hizo; lo decide el usuario.

## Suites del Proyecto 3

`p3_final_contract_tests::all_p3_suites_registered` comprueba que esta lista coincide con los archivos `tests/p3_*.rs`:

- `p3_attachment_oracle_tests`
- `p3_attachment_tests`
- `p3_benchmark_csv_tests`
- `p3_audio_tests`
- `p3_boost_motion_tests`
- `p3_clover_material_tests`
- `p3_clover_mountains_tests`
- `p3_clover_orbit_tests`
- `p3_clover_sea_tests`
- `p3_clover_surface_contract_tests`
- `p3_clover_water_animation_tests`
- `p3_collision_moving_target_tests`
- `p3_comet_tests`
- `p3_collision_sweep_tests`
- `p3_culling_toggle_tests`
- `p3_culling_work_tests`
- `p3_delivery_contract_tests`
- `p3_extent_tests`
- `p3_final_contract_tests`
- `p3_final_tour_tests`
- `p3_four_waterfalls_tests`
- `p3_frame_oracle_tests`
- `p3_frame_stats_tests`
- `p3_geometry_cache_tests`
- `p3_gravity_collision_integration_tests`
- `p3_gravity_force_tests`
- `p3_gravity_toggle_tests`
- `p3_heart_pyramid_tests`
- `p3_hearts_city_distribution_tests`
- `p3_hearts_glitch_tests`
- `p3_hearts_grid_tests`
- `p3_hearts_orbit_tests`
- `p3_hearts_terrain_tests`
- `p3_height_field_tests`
- `p3_joker_planet_tests`
- `p3_host_ownership_tests`
- `p3_hud_stats_tests`
- `p3_icosphere_topology_tests`
- `p3_identity_disc_tests`
- `p3_lightcycle_asset_tests`
- `p3_lightcycle_motion_tests`
- `p3_light_trail_tests`
- `p3_live_params_tests`
- `p3_local_terrain_collision_tests`
- `p3_lod_regression_tests`
- `p3_lod_stats_tests`
- `p3_mesh_lod_tests`
- `p3_monumental_waterfall_tests`
- `p3_motion_timestep_tests`
- `p3_neon_building_mesh_tests`
- `p3_neon_ship_lod_tests`
- `p3_neon_traffic_tests`
- `p3_orbit_layout_tests`
- `p3_orbital_capture_tests`
- `p3_orbit_speed_tests`
- `p3_procedural_scene_tests`
- `p3_prop_sector_tests`
- `p3_recording_tests`
- `p3_refactor_regression_tests`
- `p3_relative_camera_tests`
- `p3_relief_geometry_tests`
- `p3_relief_normals_tests`
- `p3_rock_mesh_tests`
- `p3_rubric_evidence_tests`
- `p3_scale_profile_tests`
- `p3_scale_ratios_tests`
- `p3_scatter_seed_tests`
- `p3_scene_spec_oracle_tests`
- `p3_ship_velocity_tests`
- `p3_ship_lights_tests`
- `p3_sky_cycle_tests`
- `p3_surface_model_rubric_tests`
- `p3_system_spec_tests`
- `p3_traffic_batch_tests`
- `p3_tree_mesh_tests`
- `p3_turn_easing_tests`
- `p3_vehicle_paths_tests`
- `p3_waterfall_geometry_tests`
- `p3_waterfall_shader_tests`
