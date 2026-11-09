# Resultados — Camino al Proyecto 3

## `baseline_shader_benchmark.csv`

Línea base medida sobre el código definitivo del Laboratorio 6 (commit `0b2b1e7`, sin cambios de producción) el 2026-11-09 con `cargo run --release -- --benchmark-shaders --benchmark-output docs/results/preproject3` y renombrado desde `shader_benchmark.csv`.

- Máquina: Intel Core i7-13650HX, 20 hilos lógicos; rustc 1.97.0; Fedora 43.
- Método: 800×600, t = 6 s, vista oblicua, 1 calentamiento + 5 rondas; mediana por fila (`src/benchmark/shaders.rs`). `threads` 20 = todos los núcleos.
- Mide solo rasterización + sombreado en CPU; no incluye `to_rgba_bytes` ni la subida de la textura a la ventana.
- Variabilidad observada en tres corridas del mismo día: ±5 % en las filas de 20 hilos y hasta ±40 % en el valor aislado `crown barycentric 20` (43.5 ms frente a 31.5 ms en otra corrida) por carga del sistema. Ninguna desviación se usa como criterio rígido; se comparan medianas de varias corridas.

## `solar_benchmark.csv`

Escenarios A–J de la escena solar (`cargo run --release -- --benchmark-solar`, `src/benchmark/solar.rs`): 800×600, t = 6 s, semilla de estrellas 42, 1 calentamiento + 7 rondas, todos los hilos (20), modo scanline. Cada ronda mide un cuadro completo de CPU (`SolarScene::render_frame`: limpieza, estrellas, cuerpos y nave); no incluye `to_rgba_bytes` ni la subida de la textura a la ventana, que solo existen con ventana.

| Escenario | Qué mide |
|---|---|
| A | Nave sola (sin cuerpos), cámara Follow |
| B | Crimson Entrance solo, rig Orbit con zoom 2.5 |
| C | Sistema completo visto de lejos con el rig Orbit |
| D | Sistema completo con la nave, cámara Follow en el punto de partida |
| E / F | Vuelo local 0.7 sobre Crimson Entrance / House of Cards, nariz 0.5 rad abajo |
| G | Cámara Follow mirando al vacío (la nave girada media vuelta) |
| H / I | D con / sin campo estelar |
| J | Vuelo local a la altura mínima (0.5) sobre Crimson Entrance, nariz 0.8 rad abajo |

Medianas de tres corridas independientes del 2026-11-09 (ms por cuadro), además del archivo commiteado, que es una cuarta corrida:

| | A | B | C | D | E | F | G | H | I | J |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Mediana de 3 corridas | 7.1 | 24.6 | 14.2 | 21.3 | 26.1 | 24.0 | 7.2 | 19.9 | 19.8 | 50.1 |
| Fragmentos sombreados | 39 380 | 30 231 | 18 356 | 57 900 | 78 182 | 101 850 | 39 380 | 57 900 | 57 900 | 150 452 |

La variación entre corridas es alta en esta laptop (hasta ±25 % en E, I y J) por el escalado de frecuencia de la CPU; H e I no se distinguen dentro de ese ruido (el campo estelar cuesta 0.02 ms medido aislado). J, el acercamiento máximo a Crimson Entrance, es el peor caso: unos 20 FPS de CPU.

## Reutilización de buffers (`RenderScratch`)

Comparación A/B intercalada en un mismo proceso (120 cuadros por variante, escenarios del benchmark solar, 800×600, 20 hilos): búferes nuevos en cada dibujo frente a `RenderScratch` reutilizado por la escena. Mediana por cuadro:

| Escenario | Búferes nuevos (ms) | Reutilizados (ms) | Cambio |
|---|---:|---:|---:|
| D | 21.68 | 21.56 | −0.5 % |
| E | 35.99 | 35.14 | −2.3 % |
| F | 30.58 | 29.02 | −5.1 % |
| J | 60.04 | 60.12 | +0.1 % |

El efecto es marginal y queda dentro del ruido entre corridas: no se atribuye una mejora de FPS. Se conserva porque las imágenes son idénticas byte a byte (oráculos y pruebas de reutilización) y evita reservar hasta unos 7 MB de muestras por instancia en cada cuadro.

## Descarte por esfera envolvente (frustum culling)

Misma metodología A/B intercalada (120 cuadros): el cuadro completo con descarte frente al mismo cuadro dibujando todos los cuerpos.

| Escenario | Cuerpos descartados | Sin descarte (ms) | Con descarte (ms) | Cambio |
|---|---:|---:|---:|---:|
| G (mirando al vacío) | 3 de 3 | 9.73 | 8.83 | −9.3 % |
| D (sistema a la vista) | 0 de 3 | 25.83 | 25.38 | −1.7 % (ruido) |

Ahorra la transformación, el recorte y el arranque de hilos de los cuerpos que no pueden aportar píxeles; cuando todo está a la vista no cambia nada. Una prueba de solidez barre 300 posiciones al azar y exige que todo cuerpo descartado produzca cero fragmentos al dibujarlo de verdad.
