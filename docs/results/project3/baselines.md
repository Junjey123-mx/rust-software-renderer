# Proyecto 3 — Huellas de cuadro congeladas

Antes de cualquier cambio del Proyecto 3 se congeló la imagen completa de la escena solar en los escenarios A–J del benchmark (`tests/p3_frame_oracle_tests.rs`).

- **Commit base:** `ea3b4a8` (sistema del preproyecto: The King's Crown, Crimson Entrance y House of Cards).
- **Configuración:** 800×600, t = 6 s, semilla de estrellas 42, relleno scanline, escenarios de `benchmark::solar::scenario_scene`.
- **Huella:** FNV-1a de 64 bits sobre los bytes RGBA de `Framebuffer::to_rgba_bytes`.
- **Determinismo:** las huellas coinciden con 1, 4 y 20 hilos y entre corridas.

| Escenario | Huella `LEGACY_A_J` |
|---|---|
| A | `0x43601441c2817a2c` |
| B | `0x6004661c1e5e8625` |
| C | `0x589f818dc4ab6f8e` |
| D | `0x186271a698e69c4b` |
| E | `0x7d10848cf33e751e` |
| F | `0x535ba2f24960e47b` |
| G | `0x7f9c2a88179af11b` |
| H | `0x186271a698e69c4b` |
| I | `0x32a8775832c50538` |
| J | `0xb8631e10f929aafc` |

D y H coinciden porque D ya incluye el campo estelar.

## Reglas

- `LEGACY_A_J` **no se reescribe nunca**: siempre se comprueba sobre `SolarScene::default_system`, que conserva el sistema del preproyecto.
- Los cambios legítimos de imagen del Proyecto 3 (reescalado, planetas nuevos) se registran en tablas nuevas, con la causa en el mensaje del commit y capturas antes y después.
- Para imprimir las huellas actuales: `P3_PRINT_BASELINE=1 cargo test --test p3_frame_oracle_tests -- --nocapture`.
