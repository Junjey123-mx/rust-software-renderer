# Proyecto 3 — Contrato del CSV del benchmark solar (versión 2)

`cargo run --release -- --benchmark-solar [--threads N] [--benchmark-output DIR]` escribe `DIR/solar_benchmark.csv`. Por omisión, `DIR` es `docs/results/preproject3`.

Cada fila es un escenario A–J (`src/benchmark/solar.rs`, `SCENARIOS`), con tiempo simulado, semilla, cámara y nave fijos. Se mide el cuadro completo en CPU (`SolarScene::render_frame`), sin la presentación en ventana.

Para registrar el commit medido:

```bash
P3_COMMIT=$(git rev-parse --short HEAD) cargo run --release -- --benchmark-solar --benchmark-output /ruta/fuera/del/repo
```

## Columnas

Las 13 primeras son las de la versión 1 (más `max_ms` y `peak_rss_kb`, que se añadieron en C02) y no cambian de orden. La versión 2 agrega nueve columnas al final.

| # | Columna | Unidad | Significado |
|---|---|---|---|
| 1 | `scenario` | — | Letra del escenario (A–J) |
| 2 | `name` | — | Descripción del escenario, sin comas |
| 3 | `width` | px | Ancho del framebuffer |
| 4 | `height` | px | Alto del framebuffer |
| 5 | `threads` | hilos | Hilos del sombreado (`--threads`; 0 = todos los núcleos, resuelto al número real) |
| 6 | `rounds` | cuadros | Cuadros medidos, sin contar el calentamiento |
| 7 | `bodies` | cuerpos | Cuerpos de la escena |
| 8 | `fragments_shaded` | muestras | Muestras del lote del último cuadro (valor de `render_frame`) |
| 9 | `median_ms` | ms | Mediana del tiempo por cuadro |
| 10 | `p95_ms` | ms | Percentil 95 |
| 11 | `min_ms` | ms | Cuadro más rápido |
| 12 | `max_ms` | ms | Cuadro más lento |
| 13 | `peak_rss_kb` | KiB | Pico de memoria residente del proceso (`VmHWM`); vacío fuera de Linux |
| 14 | `csv_version` | — | `2` |
| 15 | `objects_tested` | objetos | Objetos que pasaron por la prueba de vista: cuerpos, el grupo de lazos de plasma y la nave |
| 16 | `objects_culled` | objetos | Objetos descartados antes de transformar un solo vértice |
| 17 | `draws` | mallas | Mallas enviadas al rasterizador: cuerpos, arcos, corona, cartas, llamas y nave |
| 18 | `triangles_submitted` | triángulos | Triángulos de las mallas dibujadas |
| 19 | `triangles_culled` | triángulos | Triángulos que el rasterizador descartó: cara trasera, fuera de pantalla o delante del plano cercano |
| 20 | `lod_levels` | — | Nivel de detalle de cada cuerpo, en la forma `nombre:nivel`, separados por `;`. `-` significa que todos usan la misma esfera |
| 21 | `seed` | — | Semilla del campo de estrellas |
| 22 | `commit` | — | `P3_COMMIT`, solo letras y dígitos; vacío si no se definió |

## Reglas

- **Contadores:** las columnas 15–20 salen de `SolarScene::last_stats()` del último cuadro medido. Son exactas y no dependen del tiempo: dos corridas del mismo escenario dan los mismos valores (`tests/p3_benchmark_csv_tests.rs`). Los tiempos sí varían entre corridas.
- **Sin impresiones por fragmento:** el benchmark solo escribe una fila por escenario.
- **Lectura estricta:** un lector debe rechazar un encabezado distinto a `SOLAR_CSV_HEADER` o una fila con otro número de campos (prueba `a_row_with_a_missing_column_is_rejected`).
- **Referencia:** en G (cámara hacia el vacío), los cuerpos aportan 0 triángulos, y `triangles_submitted` es igual al de A (la nave sola: 708 triángulos de `Laboratorio3.obj` más las llamas de los propulsores).
