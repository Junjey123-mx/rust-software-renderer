# Camino al Proyecto 3 — Sistema solar navegable

Etapa preparatoria del proyecto final de **Gráficas por Computadora** (UVG, segundo ciclo 2026). Reúne en una escena nueva, `--scene solar`, los tres cuerpos del Laboratorio 6 y la nave del Laboratorio 4, con órbitas, cámara en perspectiva, pilotaje, vuelo local, fondo estelar y viaje rápido, todo con el mismo rasterizador en CPU. Raylib solo abre la ventana, lee el teclado y presenta el framebuffer.

**No es la entrega del Proyecto 3.** Su rúbrica aún no existe; esta etapa sigue las siete guías de continuación del curso (tutoriales, no requisitos) y deja una base medida y probada. Las escenas `ship`, `triangles`, `crown`, `entrance` y `house` no cambian: sus imágenes siguen congeladas por los oráculos.

## 1. Cómo ejecutar

```bash
cargo run --release -- --scene solar                       # cámara detrás de la nave
cargo run --release -- --scene solar --camera orbit        # cámara orbital ortográfica de los laboratorios
cargo run --release -- --scene solar --time 30 --set entrance.cloud_coverage=0.7
cargo run --release -- --scene solar --stars 6000 --star-density clustered
cargo run --release -- --scene solar --solar-quality adaptive
cargo run --release -- --scene solar --record 22 --fps 30 --timeline flight | ffmpeg -f rawvideo -pix_fmt rgb24 -s 800x600 -framerate 30 -i - vuelo.mp4
cargo run --release -- --benchmark-solar                   # escenarios A-J, sin ventana -> docs/results/preproject3
docs/videos/preproject3/encode.sh all                      # regenera el video, el GIF y las capturas
```

| Opción (solo `--scene solar`) | Efecto |
|---|---|
| `--camera perspective\|orbit` | Perspectiva (por defecto) u ortográfica orbital |
| `--time <s>` | Instante simulado inicial |
| `--set cuerpo.param=valor` | Parámetro de un cuerpo: `crown.`, `entrance.` o `house.` con los nombres del Laboratorio 6 |
| `--stars <n>`, `--star-seed <s>`, `--star-density uniform\|clustered` | Campo estelar; `--stars 0` lo desactiva |
| `--solar-quality full\|adaptive` | Detalle de los shaders (por defecto `full`) |
| `--record <s> [--fps n] [--timeline flight]` | Vuelo guionizado en rgb24 por stdout, sin ventana |
| `--threads <n>`, `--mode`, `--view`, `--screenshot` | Igual que en las escenas del Laboratorio 6 |

### Controles

| Tecla | Rig Follow (por defecto) | Rig Orbit |
|---|---|---|
| `W` / `S` | Avanzar / retroceder | — |
| `LEFT SHIFT` | Velocidad de la luz (×8, solo vuelo espacial) | — |
| `←` `→` | Girar (en vuelo local, alrededor del arriba radial) | Orbitar la cámara |
| `↑` `↓` | Levantar / bajar la nariz | Inclinar la cámara |
| `L` | Entrar o salir del vuelo local sobre el planeta más cercano | — |
| `E` / `Q` | Alejarse / acercarse al planeta (vuelo local) | Zoom |
| `F1` `F2` `F3` | Warp junto a The King's Crown, Crimson Entrance o House of Cards | Igual |
| `V` | Cambiar al rig Orbit | Volver al rig Follow |
| `SPACE` | Pausar órbitas, giros y materiales (la nave sigue volando) | Igual |
| `1` `2` `3` | Wireframe, baricéntrico, scanline | Igual |
| `R` `T` `F` `S` | — | Reiniciar y vistas predefinidas |

El título de la ventana muestra el modo de vuelo, el planeta seguido o el más cercano con la altitud sobre su superficie ideal, la velocidad, el rig y los FPS.

## 2. Arquitectura (alternativa D, híbrida)

Una infraestructura de render compartida, una escena plana de instancias y módulos pequeños; sin ECS ni grafo de escena. Todo es aditivo: ningún camino anterior cambió de comportamiento.

| Módulo | Responsabilidad |
|---|---|
| `src/pipeline/transform.rs` | `Transform`: matriz T·R·S en el layout del vertex shader e inversa de direcciones |
| `src/pipeline/perspective.rs` | `PerspectiveCamera`, `ClipVertex`, recorte contra el plano cercano y prueba de esfera contra el frustum |
| `src/pipeline/shaded.rs` | Etapa sombreada: camino orbital del Laboratorio 6 (bit a bit igual), camino en perspectiva, `RenderScratch` y lote por cuadro |
| `src/solar/resources.rs` | Esfera y nave cargadas una vez y compartidas (`Arc`) |
| `src/solar/instance.rs` | `BodyInstance`, uniforms en espacio de objeto, sombreado con el ojo por fragmento, calidad adaptativa |
| `src/solar/clock.rs`, `orbit.rs` | Reloj de simulación y órbitas circulares en forma cerrada |
| `src/solar/scene.rs` | `SolarScene`: disposición, luz desde la estrella, cámaras, descarte, cuadro por lotes |
| `src/solar/ship.rs`, `flight.rs`, `camera.rs`, `input.rs` | Nave con base ortonormal, vuelo espacial y local, rigs Follow/Orbit, teclado propio |
| `src/solar/skybox.rs`, `travel.rs` | Campo estelar determinista y viaje rápido |
| `src/solar/viewer.rs`, `record.rs` | Ventana y grabación sin ventana |
| `src/benchmark/solar.rs` | Escenarios A–J |

### Contrato del cuadro

Entrada → reloj → órbitas y giros → controlador activo (uno solo: vuelo espacial, vuelo local o cámara orbital) → cámara → **una** limpieza de color y profundidad → estrellas (solo color) → cuerpos → nave → presentación. Todos los objetos comparten el framebuffer, la profundidad y la cámara del cuadro.

### Espacios y convenciones

- Los shaders del Laboratorio 6 trabajan en **espacio de objeto**: la luz y el ojo de mundo se giran con la inversa de la rotación de cada instancia, y la posición interpolada es la original de la malla, así que picas, rombos y bucles quedan pegados al cuerpo cuando gira.
- **Perspectiva:** vista `(x derecha, y arriba, z adelante)`, profundidad = `z` de vista (menor es más cerca). El recorte contra el plano cercano ocurre antes de dividir por `z` y cada vértice nuevo interpola todos sus atributos; los pesos son correctos en perspectiva (`λᵢ/zᵢ` normalizados) y la profundidad es `1/Σ(λᵢ/zᵢ)`. Fresnel, especular y parallax usan la dirección al ojo **de cada fragmento**.
- **Nave:** `Laboratorio3.obj` se midió antes de fijar ejes: nariz hacia −Z, alas en X, arriba +Y, centro de la caja en (0, 0.255, −1.3), radio 3.99. Escala 0.08 en la escena.
- **Luz:** cada planeta recibe la dirección hacia la estrella; la estrella es emisiva.

### Disposición por defecto

| Cuerpo | Escala (radio) | Órbita | Periodo orbital | Giro propio |
|---|---|---|---|---|
| The King's Crown | 1.3 (1.13) | centro | — | 0.05 rad/s |
| Crimson Entrance | 0.6 (0.52) | radio 2.6, 0.5 rad/s | 12.6 s | 0.6 rad/s |
| House of Cards | 0.9 (0.78) | radio 4.4, 0.22 rad/s, fase π | 28.6 s | 0.35 rad/s |

La nave parte en (0, 0.6, 7.5) mirando a la estrella.

## 3. Las siete guías y lo que implementa cada una

| Guía | Implementado | No implementado (y por qué) |
|---|---|---|
| I. Múltiples modelos | Instancias con transformación y material propios sobre una malla compartida; una limpieza y una profundidad por cuadro; oclusión independiente del orden | — |
| II. Movimiento de objetos | Reloj único con pausa y velocidad; órbitas y giros en función del tiempo acumulado; luz desde la estrella | — |
| III. Movimiento de la nave | Nave pilotable independiente de los FPS, modelo orientado y cámara detrás | La orientación usa una base `forward/up` desde el inicio en lugar de yaw/pitch, para no migrar en la guía IV. Las extensiones opcionales (mirada libre y cámara suavizada) no se hicieron |
| IV. Acercarse a un planeta | Vuelo local con arcos de círculo máximo a distancia constante, seguimiento del desplazamiento orbital, altitud con holgura mínima, cambio de modo sin saltos | Sin gravedad ni colisiones, como indica la guía |
| V. Skybox | Direcciones deterministas proyectadas con la rotación de la cámara, solo color, ocultas por los cuerpos; densidad por ruido opcional y tinte por estrella | Estrellas de 2–3 píxeles no implementadas |
| VI. Optimización | Benchmark A–J, reutilización de búferes, descarte por esfera, calidad adaptativa opcional y planificador por cuadro (sección 4) | LOD de malla descartado: la esfera tiene 192 triángulos y el coste está en los fragmentos. Sin profiler externo (`perf` bloqueado sin sudo); se midió con temporizadores y A/B |
| VII. Quick Travel | Warp a la posición actual del destino con llegada segura y velocidad de la luz | Mapa y clic, reservado para cuando exista la rúbrica |

## 4. Rendimiento medido

Escenarios A–J sin ventana (`--benchmark-solar`, 800×600, 20 hilos, mediana de 7 rondas tras 1 de calentamiento; cuadro completo de CPU sin la presentación). Valores de `docs/results/preproject3/solar_benchmark.csv`, una corrida con el código final:

| | A nave sola | B un planeta | C sistema lejano | D sistema con nave | E cerca de Entrance | F cerca de House | G vacío | H con estrellas | I sin estrellas | J vuelo mínimo |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| ms | 6.83 | 23.19 | 6.91 | 15.66 | 19.65 | 13.50 | 7.20 | 16.51 | 15.73 | 51.91 |
| Fragmentos | 26 324 | 30 128 | 18 319 | 44 844 | 65 126 | 88 793 | 26 324 | 44 844 | 44 844 | 137 396 |

La variación entre corridas llega a ±25 % en esta laptop, por el escalado de frecuencia. Por eso cada decisión de optimización se tomó con A/B intercalados en un mismo proceso (detalles en [docs/results/preproject3/README.md](results/preproject3/README.md)):

| Commit | Cambio | Resultado medido | Decisión |
|---|---|---|---|
| C34 | Reutilizar búferes de muestras y colores | Entre +0.1 % y −5.1 %, dentro del ruido | Se conserva por seguridad de memoria; no se atribuye mejora |
| C35 | Descarte por esfera contra el frustum | −9.3 % mirando al vacío; sin cambio con todo a la vista | Adoptado |
| C36 | Calidad adaptativa opcional | −5.3 % y −3.3 % en vistas lejanas, con el 1.5 % de píxeles cambiados; acercamientos idénticos | Opcional, apagado por defecto |
| C37 | Un solo grupo de hilos por cuadro | −26.3 % en D, −31.5 % en C, sin regresión en ningún escenario, 48 imágenes idénticas | Adoptado ([estudio](results/preproject3/thread_scheduler_study.md)) |

El peor caso es J, el vuelo a la altura mínima sobre Crimson Entrance: unos 52 ms (≈ 19 FPS de CPU), dominado por el parallax y el bump del shader.

## 5. Pruebas

`cargo test` ejecuta **551 pruebas**: las 402 anteriores, sin modificar, y 149 nuevas en 17 archivos `tests/preproject3_*.rs` y `tests/solar_*.rs`. Destacan:

- `preproject3_oracle_tests`: huellas FNV de los tres cuerpos del Laboratorio 6, fijadas antes de tocar el pipeline.
- `solar_perspective_tests`: proyección, recorte en todos los casos, pesos en perspectiva y profundidad real; la ruta orbital por `render_shaded_with` es bit a bit igual a `render_shaded`.
- `solar_scene_tests`: oclusión independiente del orden, luz desde la estrella, solidez del descarte (300 posiciones al azar) e imagen del lote idéntica al dibujo por instancia (48 combinaciones).
- `solar_flight_tests`: independencia de los FPS, distancia constante en media vuelta, seguimiento orbital, holgura y estabilidad en los polos.
- `solar_contract_tests`: sin dibujo 3D, carga de modelos ni texturas de raylib; dependencias y OBJ intactos.

## 6. Evidencias

- Video: [docs/videos/preproject3/01-solar-flight.mp4](videos/preproject3/01-solar-flight.mp4), H.264 a 800×600 y 30 fps, 22 s y 660 cuadros. GIF: [01-solar-flight.gif](videos/preproject3/01-solar-flight.gif), 320×240, 176 cuadros.
- Capturas en [docs/screenshots/preproject3/](screenshots/preproject3/): `01-system-orbit`, `02-ship-follow`, `03-local-flight`, `04-skybox` y `05-quick-travel`.
- Todo sale del renderer real: `encode.sh` graba con `--record`, pasa por un intermedio FFV1 sin pérdida y verifica códec, tamaño, cuadros y decodificación completa antes de reemplazar.

## 7. Límites

- Sin colisiones ni gravedad: con la velocidad de la luz la nave puede atravesar un cuerpo, y la cámara Follow puede quedar dentro de un planeta si se pilota a propósito contra él.
- El sombreado de la nave es plano por cara (luz direccional del curso), no un material.
- La presentación (`to_rgba_bytes` y la subida de la textura) no está en el benchmark; solo existe con ventana.
- La silueta de los cuerpos es poligonal (192 triángulos) y se nota de cerca.

## 8. Ajustes respecto al plan del Gate 10

- **C06:** el campo `orbit` de `BodyInstance` llegó en C12, cuando ya existía `Orbit` (C11).
- **C20:** `render_ship` reutiliza `intensity` del sombreado plano dentro de la etapa sombreada en perspectiva, en vez de añadir `shade_faces_object_light` y `render_interactive_with`. `face_shading.rs` y `render.rs` no se tocaron.
- **C22:** la perspectiva del rig Orbit se movió a `camera.rs` y la escena la reutiliza.
- **C30:** la densidad por ruido se activa con `--star-density clustered`; por defecto el cielo es uniforme.
- **C36:** sin histéresis. Una rampa continua sin estado mantiene determinista cada instante, algo necesario para grabaciones y benchmarks.
- **C39:** `--timeline flight` convive con las timelines del Laboratorio 6 sin modificar su enum.
- **Esfera:** `sphere.obj` no es unitaria (radio 0.840–0.866). Los radios de planeta, la altitud y el descarte usan 0.866.
