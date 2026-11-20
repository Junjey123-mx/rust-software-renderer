# Proyecto 3 — Contrato de entrega (requisitos habilitantes R1–R5)

El PDF oficial del Proyecto 3 fija cinco requisitos habilitantes: **si falta uno, el proyecto vale 0**. Esta tabla los convierte en un contrato verificable.

- `tests/p3_delivery_contract_tests.rs` lee esta tabla.
- Toda fila en `CUMPLE` debe apuntar a evidencia que exista en el repositorio.
- Las filas `PENDIENTE` declaran lo que todavía falta.

**Una prueba verde no sustituye la revisión docente**: el profesor decide si cada requisito se cumple.

| Req | Requisito (PDF) | Estado | Evidencia | Demostración |
|---|---|---|---|---|
| R1 | Pipeline propio: raylib solo para ventana, entrada y mostrar el framebuffer; sin dibujo 3D, cámaras, shaders de GPU ni física de raylib | CUMPLE | `Cargo.toml`; `src/pipeline/shaded.rs`; `src/rasterization/mod.rs`; `src/rendering/presenter.rs` | `cargo run --release -- --scene solar` |
| R2 | Sol, nave propia y al menos un planeta; la nave se mueve bajo control del jugador; el planeta orbita y rota; todo depende del tiempo transcurrido | CUMPLE | `src/solar/scene.rs`; `src/solar/orbit.rs`; `src/solar/clock.rs`; `src/solar/flight.rs`; `assets/models/Laboratorio3.obj` | `cargo run --release -- --scene solar` (W A S D, ratón, SPACE) |
| R3 | Acercarse para apreciar la superficie y alejarse sin pantalla de carga ni cambio de escena, pudiendo orientar la nave | CUMPLE | `src/solar/flight.rs`; `src/solar/camera.rs` | Volar hacia un planeta, `L` (vuelo local) y `L` para salir |
| R4 | Modelos 3D con transformaciones de modelo, vista y proyección, prueba de profundidad y al menos un shader de fragmentos propio | CUMPLE | `src/io/obj.rs`; `src/pipeline/perspective.rs`; `src/rendering/framebuffer.rs`; `src/celestial/crown.rs` | `cargo run --release -- --scene solar` |
| R5 | Código, modelos e instrucciones; video corto del recorrido completo (vuelo espacial, acercamiento, vuelta alrededor de un planeta y alejamiento); README con integrantes, contribuciones, características reclamadas y declaración de IA y recursos externos | PENDIENTE | `README.md`; `docs/videos/preproject3/01-solar-flight.mp4` | Falta un recorrido continuo **sin warp** con una vuelta completa, y falta la sección de entrega del README |

## Pendientes conocidos de R5

1. **Video:** el video del preproyecto (`docs/videos/preproject3/01-solar-flight.mp4`, 22 s) llega a los planetas con **warp** y termina cambiando de cámara, así que no cumple el recorrido continuo. Se resuelve con el guion `--timeline tour` (C04) y su clip (C05).
2. **README:** falta la sección de entrega del Proyecto 3: integrantes y contribuciones, lista de características reclamadas con dónde verlas, y declaración de asistencia de IA y recursos externos (C05, C79).
