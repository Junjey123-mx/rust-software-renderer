# Camino al Proyecto 3 — Sistema solar navegable

Etapa preparatoria del proyecto final de **Gráficas por Computadora** (UVG, segundo ciclo 2026). Reúne en una escena nueva, `--scene solar`, los tres cuerpos del Laboratorio 6 y la nave del Laboratorio 4, con órbitas, cámara en perspectiva, pilotaje, vuelo local, un cielo Red-Black (nebulosa, estrellas de colores y cometas) y viaje rápido, todo con el mismo rasterizador en CPU. Raylib solo abre la ventana, lee el teclado y presenta el framebuffer.

**No es la entrega del Proyecto 3.** Su rúbrica aún no existe; esta etapa sigue las siete guías de continuación del curso (tutoriales, no requisitos) y deja una base medida y probada. Las escenas `ship`, `triangles`, `crown`, `entrance` y `house` no cambian: sus imágenes siguen congeladas por los oráculos.

## 1. Cómo ejecutar

```bash
cargo run --release -- --scene solar                       # cámara detrás de la nave
cargo run --release -- --scene solar --camera orbit        # cámara orbital ortográfica de los laboratorios
cargo run --release -- --scene solar --time 30 --set entrance.cloud_coverage=0.7
cargo run --release -- --scene solar --nebula 0.5 --star-seed 7      # cielo más tenue, otra semilla
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
| `--stars <n>`, `--star-seed <s>`, `--star-density uniform\|clustered` | Estrellas y cometas: por defecto 8000 candidatos agrupados (unas 4000 estrellas); `--stars 0` los desactiva. La semilla también fija la nebulosa |
| `--nebula <0-1>` | Intensidad de la nebulosa (por defecto 1; 0 deja el cielo negro) |
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
| `src/solar/skybox.rs`, `nebula.rs`, `travel.rs` | Estrellas y cometas deterministas, nebulosa precalculada y viaje rápido |
| `src/solar/plasma_loops.rs` | Lazos de plasma externos de The King's Crown: arcos, tubos precalculados y sombreado emisivo (sección 10) |
| `src/solar/viewer.rs`, `record.rs` | Ventana y grabación sin ventana |
| `src/benchmark/solar.rs` | Escenarios A–J |

### Contrato del cuadro

Entrada → reloj → órbitas y giros → controlador activo (uno solo: vuelo espacial, vuelo local o cámara orbital) → cámara → **una** limpieza de color y profundidad → nebulosa, cometas y estrellas (solo color) → cuerpos → nave → presentación. Todos los objetos comparten el framebuffer, la profundidad y la cámara del cuadro.

### Espacios y convenciones

- Los shaders del Laboratorio 6 trabajan en **espacio de objeto**: la luz y el ojo de mundo se giran con la inversa de la rotación de cada instancia, y la posición interpolada es la original de la malla, así que picas, rombos y bucles quedan pegados al cuerpo cuando gira.
- **Perspectiva:** vista `(x derecha, y arriba, z adelante)`, profundidad = `z` de vista (menor es más cerca). El recorte contra el plano cercano ocurre antes de dividir por `z` y cada vértice nuevo interpola todos sus atributos; los pesos son correctos en perspectiva (`λᵢ/zᵢ` normalizados) y la profundidad es `1/Σ(λᵢ/zᵢ)`. Fresnel, especular y parallax usan la dirección al ojo **de cada fragmento**.
- **Nave:** `Laboratorio3.obj` se midió antes de fijar ejes: nariz hacia −Z, alas en X, arriba +Y, centro de la caja en (0, 0.255, −1.3), radio 3.99. Escala 0.08 en la escena.
- **Luz:** cada planeta recibe la dirección hacia la estrella; la estrella es emisiva.

### Disposición por defecto

| Cuerpo | Escala (radio) | Órbita | Periodo orbital | Giro propio |
|---|---|---|---|---|
| The King's Crown | 1.3 (1.13) | centro | — | 0.05 rad/s |
| Crimson Entrance | 0.6 (0.52) | radio 3.0, 0.5 rad/s | 12.6 s | 0.6 rad/s |
| House of Cards | 0.9 (0.78) | radio 6.2, 0.22 rad/s, fase π | 28.6 s | 0.35 rad/s |

La nave parte en (0, 0.6, 9.0) mirando a la estrella. Entre las dos órbitas queda espacio para un futuro anillo de hasta 2.2 radios alrededor de House of Cards (sección 9).

## 3. Las siete guías y lo que implementa cada una

| Guía | Implementado | No implementado (y por qué) |
|---|---|---|
| I. Múltiples modelos | Instancias con transformación y material propios sobre una malla compartida; una limpieza y una profundidad por cuadro; oclusión independiente del orden | — |
| II. Movimiento de objetos | Reloj único con pausa y velocidad; órbitas y giros en función del tiempo acumulado; luz desde la estrella | — |
| III. Movimiento de la nave | Nave pilotable independiente de los FPS, modelo orientado y cámara detrás | La orientación usa una base `forward/up` desde el inicio en lugar de yaw/pitch, para no migrar en la guía IV. Las extensiones opcionales (mirada libre y cámara suavizada) no se hicieron |
| IV. Acercarse a un planeta | Vuelo local con arcos de círculo máximo a distancia constante, seguimiento del desplazamiento orbital, altitud con holgura mínima, cambio de modo sin saltos | Sin gravedad ni colisiones, como indica la guía |
| V. Skybox | Direcciones deterministas proyectadas con la rotación de la cámara, solo color, ocultas por los cuerpos; densidad por ruido; cinco clases espectrales con estrellas en cruz y destellos; nebulosa y cometas (sección 9) | Las estelas son fijas: no hay estrellas fugaces animadas |
| VI. Optimización | Benchmark A–J, reutilización de búferes, descarte por esfera, calidad adaptativa opcional y planificador por cuadro (sección 4) | LOD de malla descartado: la esfera tiene 192 triángulos y el coste está en los fragmentos. Sin profiler externo (`perf` bloqueado sin sudo); se midió con temporizadores y A/B |
| VII. Quick Travel | Warp a la posición actual del destino con llegada segura y velocidad de la luz | Mapa y clic, reservado para cuando exista la rúbrica |

## 4. Rendimiento medido

Escenarios A–J sin ventana (`--benchmark-solar`, 800×600, 20 hilos, mediana de 7 rondas tras 1 de calentamiento; cuadro completo de CPU sin la presentación). Valores de `docs/results/preproject3/solar_benchmark.csv`, una corrida al cierre del Gate 10, antes del cielo Red-Black (su costo está en la sección 9):

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

`cargo test` ejecuta **574 pruebas**: las 402 de los laboratorios, sin modificar, y 172 de esta etapa en 19 archivos `tests/preproject3_*.rs` y `tests/solar_*.rs`. Destacan:

- `preproject3_oracle_tests`: huellas FNV de los tres cuerpos del Laboratorio 6, fijadas antes de tocar el pipeline.
- `solar_perspective_tests`: proyección, recorte en todos los casos, pesos en perspectiva y profundidad real; la ruta orbital por `render_shaded_with` es bit a bit igual a `render_shaded`.
- `solar_scene_tests`: oclusión independiente del orden, luz desde la estrella, solidez del descarte (300 posiciones al azar) e imagen del lote idéntica al dibujo por instancia (48 combinaciones).
- `solar_flight_tests`: independencia de los FPS, distancia constante en media vuelta, seguimiento orbital, holgura y estabilidad en los polos.
- `solar_contract_tests`: sin dibujo 3D, carga de modelos ni texturas de raylib; dependencias y OBJ intactos.
- `solar_nebula_tests` y `solar_skybox_tests`: paleta acotada, fidelidad del mapa, invariancia ante la traslación de la cámara, escritura solo de color, clases espectrales, tamaños y cometas.
- `solar_plasma_tests`: pies enterrados y ápice fuera de la silueta, tubos orientados hacia afuera, oclusión por la estrella, colores cálidos y geometría estable que gira con la estrella (sección 10).

## 6. Evidencias

- Video: [docs/videos/preproject3/01-solar-flight.mp4](videos/preproject3/01-solar-flight.mp4), H.264 a 800×600 y 30 fps, 22 s y 660 cuadros. GIF: [01-solar-flight.gif](videos/preproject3/01-solar-flight.gif), 320×240, 176 cuadros.
- Capturas en [docs/screenshots/preproject3/](screenshots/preproject3/): `01-system-orbit`, `02-ship-follow`, `03-local-flight`, `04-skybox` y `05-quick-travel`.
- Todo sale del renderer real: `encode.sh` graba con `--record`, pasa por un intermedio FFV1 sin pérdida y verifica códec, tamaño, cuadros y decodificación completa antes de reemplazar.

## 7. Límites

- Sin colisiones ni gravedad: con la velocidad de la luz la nave puede atravesar un cuerpo, y la cámara Follow puede quedar dentro de un planeta si se pilota a propósito contra él.
- El sombreado de la nave es plano por cara (luz direccional del curso), no un material.
- La presentación (`to_rgba_bytes` y la subida de la textura) no está en el benchmark; solo existe con ventana.
- La silueta de los cuerpos es poligonal (192 triángulos) y se nota de cerca.
- La nebulosa se dibuja en un solo hilo (unos 1.7 ms por cuadro) y su mapa se pinta en el primer cuadro (unos 20 ms una vez).

## 8. Ajustes respecto al plan del Gate 10

- **C06:** el campo `orbit` de `BodyInstance` llegó en C12, cuando ya existía `Orbit` (C11).
- **C20:** `render_ship` reutiliza `intensity` del sombreado plano dentro de la etapa sombreada en perspectiva, en vez de añadir `shade_faces_object_light` y `render_interactive_with`. `face_shading.rs` y `render.rs` no se tocaron.
- **C22:** la perspectiva del rig Orbit se movió a `camera.rs` y la escena la reutiliza.
- **C30:** la densidad por ruido se activaba con `--star-density clustered` y el cielo era uniforme por defecto; desde la sección 9 el valor por defecto es agrupado.
- **C36:** sin histéresis. Una rampa continua sin estado mantiene determinista cada instante, algo necesario para grabaciones y benchmarks.
- **C39:** `--timeline flight` convive con las timelines del Laboratorio 6 sin modificar su enum.
- **Esfera:** `sphere.obj` no es unitaria (radio 0.840–0.866). Los radios de planeta, la altitud y el descarte usan 0.866.

## 9. Después del Gate: cielo Red-Black y órbitas para el anillo

El cielo negro con 2000 puntos blancos se sentía vacío, y el sistema estaba apretado: entre Crimson Entrance y House of Cards quedaban 0.5 unidades, y un anillo alrededor del gigante gaseoso habría cruzado la órbita de Crimson Entrance.

**Órbitas.** Crimson Entrance pasa de 2.6 a 3.0 y House of Cards de 4.4 a 6.2, con las mismas velocidades angulares.
- Separaciones: 1.35 entre la estrella y Crimson Entrance, y casi una unidad entre Crimson Entrance y un anillo de 2.2 radios alrededor de House of Cards (1.7 unidades).
- La nave parte en z = 9.0, fuera de esa reserva.
- La cámara orbital encuadra 8.0 en vez de 5.3 para que el anillo quepa.
- Una prueba fija la reserva, la separación interior, el encuadre y el punto de partida.

**Nebulosa** (`src/solar/nebula.rs`). La paleta es la del cielo `red_black` del proyecto Red-Black: negro violáceo, púrpura profundo, vino, corinto y magenta oscuro.
- Composición: un gradiente vertical (negro violáceo arriba, vino abajo) y una banda inclinada de nubes púrpura y magenta formada por dos capas de `fbm`, con huecos oscuros.
- Brillo: el canal más alto no pasa de 0.5 y la luminancia media es baja, para que los cuerpos sigan siendo lo principal.
- Precálculo: los colores se pintan una sola vez en un mapa de longitud-latitud de 256×128.
- Por cuadro: el mapa se muestrea con la rotación de la cámara cada 8 píxeles y se interpola por celdas, así que el ruido nunca se evalúa por píxel.
- Profundidad: solo escribe color, así que la oclusión y el lote por cuadro no cambian.

**Estrellas.** Las estrellas tienen cinco clases espectrales estilizadas: azul-blanca 18 %, blanca 27 %, amarilla 22 %, naranja 18 % y roja 15 %.
- Tamaño: según el brillo, cerca del 4 % se dibuja en cruz de 5 píxeles y menos del 1 % como destello de brazos de 3 píxeles.
- Mezcla: suman su luz al fondo.
- Densidad: por defecto, 8000 candidatos agrupados por ruido (unas 4000 estrellas).
- Cometas: doce estelas fijas, de 4 a 9 grados y azul pálido, que se desvanecen de la cabeza a la cola; suele verse una por cuadro.
- Semilla: estrellas, cometas y nebulosa salen de la misma `--star-seed`, y las direcciones y brillos de cada semilla no cambiaron.

**Costo medido** (A/B intercalado contra `c0a06a4`, tres corridas por lado, laptop con batería y perfil `quiet`):
- **Cielo completo** (escenario H menos I): de 0.35 a 2.04 ms por cuadro.
- **Escenas ligeras** (nave sola, cámara al vacío): +1.5 ms (≈ +23 %).
- **Escenas con planetas:** entre +2 % y +7 %.
- **Sin cielo** (I): −12 %, porque las órbitas más abiertas dejan menos fragmentos.

`solar_benchmark.csv` conserva la medición del Gate como evidencia histórica.

**Pruebas que cambiaron con el comportamiento:**
- **Una estrella, un píxel:** la prueba que lo asumía ahora admite el tamaño de los destellos.
- **Fondo negro:** la prueba de una sola limpieza compara la esquina con un cuadro nuevo en vez de con el negro.
- **Lote por cuadro:** la referencia de la prueba de equivalencia pinta también la nebulosa.
- **CLI:** pide `--star-density uniform` donde exige un número exacto de estrellas.

El escenario I del benchmark quita ahora todo el cielo, para medirlo como H − I.

## 10. Lazos de plasma externos de The King's Crown

En el Laboratorio 6 los lazos de la estrella son trazos luminosos que el shader pinta dentro del disco. En el sistema solar ahora son geometría: seis arcos magnéticos que nacen en un pie, salen de la silueta, se arquean en el espacio y vuelven a entrar por otro pie.

**Superficie y fenómenos externos.**
- La superficie sigue en `src/celestial/crown.rs` (plasma, manchas, corona del limbo y eclipse) y no cambió: los oráculos siguen congelando las imágenes de `--scene crown`.
- Los fenómenos externos viven en `src/solar/plasma_loops.rs`, que solo usa la escena solar.
- En `--scene solar` el shader de superficie deja de pintar sus lazos (`loops = 0`) para no tener dos versiones del mismo fenómeno; `--set crown.loops=1` los recupera.

**Datos** (`PlasmaArc`, `CROWN_ARCS`).
- Cada arco guarda su punto medio (longitud y latitud), la inclinación y la separación angular de sus pies, la altura del ápice y el grosor del tubo en radios del cuerpo, un desplazamiento en la rampa de color, su intensidad y su fase.
- Los seis arcos se reparten cada 60° de longitud en cinturones activos alternos: desde cualquier lado, unos dos quedan cerca del limbo y muestran el arco completo contra el cielo.

**Geometría.**
- El eje sigue el círculo máximo entre los pies y se eleva con un seno, desde pies enterrados al 88 % del radio hasta un ápice fuera de la silueta. Todo el eje queda en un plano que pasa por el centro de la estrella.
- Un tubo de 14 segmentos y 5 lados por arco (840 triángulos en total; la esfera tiene 192) se construye una sola vez en espacio de objeto y se comparte con `Arc`.
- Se dibuja con la transformación de la estrella, así que gira con ella, y comparte el búfer de profundidad: la estrella oculta la parte trasera de los arcos, y los planetas y la nave los tapan.

**Integración.**
- Cada arco es un `Job::Plasma` con su propia etiqueta en el lote por cuadro.
- También se dibuja por instancia (`render_plasma_without_clear`) y en wireframe, en dorado; la prueba de equivalencia del lote lo incluye.
- El descarte usa la esfera de la estrella agrandada por el alcance de los lazos.

**Sombreado emisivo y animado** (`ArcShader::shade`).
- **Normal suave:** el punto del eje más cercano a cada fragmento (ángulo en el plano del arco y un paso de Newton) da la normal del tubo, sin importar las facetas.
- **Color:** el núcleo que mira al ojo arde de dorado a blanco caliente y el borde se enfría a vino y carmesí, con la rampa de plasma de la propia estrella.
- **Vida:** hebras torcidas como una cuerda de flujo magnético, grano de ruido y nudos brillantes que corren de un pie al otro, pies más calientes y un pulso de 5 s con fase propia por arco.
- **Estabilidad:** la geometría no cambia con el tiempo, solo el color; una prueba lo comprueba con la profundidad.

**Costo medido** (A/B intercalado contra `6a73efa`, cuatro corridas por lado, la misma laptop con perfil `quiet`):
- Con la estrella a la vista (escenarios C, D, H e I) los lazos suman entre 0.2 y 0.7 ms por cuadro (+1 % a +5 %).
- En proceso, el escenario I con y sin arcos difiere en unos 0.4 ms. La mayor parte es rasterizar triángulos pequeños, que va en un solo hilo; el sombreado se reparte en el grupo de hilos del lote.
- Los escenarios sin la estrella sombrean los mismos fragmentos y sus diferencias son ruido.

Las capturas y el video de la sección 6 son anteriores a los lazos.

**Cómo escalar** (no implementado):
- **Corona 3D:** una cáscara alrededor de la estrella necesita mezcla aditiva o alfa, que el framebuffer todavía no tiene. El lote ya admite otro tipo de `Job`.
- **Fulguraciones:** animar la altura de un arco regenerando solo su tubo (140 triángulos) en el cuadro en que cambia.
- **Eyección de partículas:** puntos en el espacio de objeto de la estrella, proyectados como las estrellas del cielo pero con profundidad.
- **Halos y bloom:** un posproceso sobre el framebuffer en flotante, donde los colores ya pueden pasar de 1.
- **Detalle por distancia:** un tubo más simple cuando la estrella es pequeña en pantalla.
