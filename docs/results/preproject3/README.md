# Resultados — Camino al Proyecto 3

## `baseline_shader_benchmark.csv`

Línea base medida sobre el código definitivo del Laboratorio 6 (commit `0b2b1e7`, sin cambios de producción) el 2026-11-09 con `cargo run --release -- --benchmark-shaders --benchmark-output docs/results/preproject3` y renombrado desde `shader_benchmark.csv`.

- Máquina: Intel Core i7-13650HX, 20 hilos lógicos; rustc 1.97.0; Fedora 43.
- Método: 800×600, t = 6 s, vista oblicua, 1 calentamiento + 5 rondas; mediana por fila (`src/benchmark/shaders.rs`). `threads` 20 = todos los núcleos.
- Mide solo rasterización + sombreado en CPU; no incluye `to_rgba_bytes` ni la subida de la textura a la ventana.
- Variabilidad observada en tres corridas del mismo día: ±5 % en las filas de 20 hilos y hasta ±40 % en el valor aislado `crown barycentric 20` (43.5 ms frente a 31.5 ms en otra corrida) por carga del sistema. Ninguna desviación se usa como criterio rígido; se comparan medianas de varias corridas.
