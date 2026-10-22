# Laboratorio 5 — Filling triangles

Guía técnica del trabajo implementado en la rama `lab5/triangle-filling`, que
extiende el Laboratorio 4 (`docs/laboratorio4.md`) sin modificar su contorno.

## Objetivo oficial

Ampliar `triangle`, que dibuja los tres bordes, para que el renderizador también
genere fragmentos del interior con **dos algoritmos independientes**
(coordenadas baricéntricas y scanline), conservar el modo de contorno, comparar
las posiciones producidas por ambos rellenos, medir solo la generación de
fragmentos en modo optimizado y usar un perfilador para ver dónde se consume
el tiempo.

## Trazabilidad requisito → evidencia

| ID | Requisito del enunciado | Evidencia en el repositorio |
|---|---|---|
| R01 | `Wireframe` con las tres llamadas a `line` | `rasterize_with_mode(.., Wireframe, ..) == rasterize(..)`; `tests/lab5_baseline_tests.rs`, `tests/rasterizer_modes_tests.rs` |
| R02 | `FilledBarycentric` independiente | `src/rasterization/barycentric.rs`; `tests/barycentric_tests.rs` |
| R03 | `FilledScanline` independiente, una fila a la vez | `src/rasterization/scanline.rs`; `tests/scanline_tests.rs` |
| R04 | `TriangleMode` pasado a `rasterize` sin tocar ensamblaje ni shader | `src/rasterization/mode.rs`, `src/pipeline/rasterizer.rs`, `src/pipeline/render.rs` |
| R05 | Bounding box y muestreo en `(x+0.5, y+0.5)` | `coverage::center_range`, `barycentric::fill_barycentric` |
| R06 | Orden inverso no borra el triángulo; área cero segura | `coverage::orient`; pruebas de winding y degenerados en todos los archivos de test |
| R07 | Scanline consistente en horizontales y vértices | `scanline::row_crossings` (semiabierto `[min_y, max_y)` + regla top-left) |
| R08 | Casos pequeño/grande/horizontal/vertical/invertido/recortado/colineal | matriz T01–T09 en `tests/triangle_coverage_tests.rs`; estrés en `tests/triangle_edge_cases_tests.rs` |
| R09 | Comparar posiciones `(x, y)`, no imágenes | `assert_parity` con diff `only_in_barycentric` / `only_in_scanline` |
| R10 | Medir solo fragmentos, `Instant`, release, repeticiones, mediana | `src/benchmark/triangles.rs`; `--benchmark-triangles` |
| R11 | Contar fragmentos por algoritmo | columna `fragments_per_iteration` en los CSV; paridad verificada antes de medir |
| R12 | Perfilador | `docs/profiles/` (muestreo de pilas con `eu-stack`) |
| R13 | Una imagen por modo | `docs/screenshots/lab5/01-wireframe.png`, `02-barycentric.png`, `03-scanline.png` |
| R14 | Tiempos, perfil y explicación | este documento y el README |

## Arquitectura heredada y lo que se añadió

```text
Mesh / escena  → vertex_shader (+ fit)  → primitive_assembly
                                               │
                              rasterize_with_mode(triangles, mode, bounds)
                                   ┌────────────┼────────────┐
                              Wireframe   FilledBarycentric  FilledScanline
                              triangle    fill_barycentric   fill_scanline
                              → 3×line    bbox + pesos       cortes por fila
                                   └────────────┼────────────┘
                                          Vec<Fragment>
                                               │
                              fragment_shader (sin cambios) → Framebuffer::point
```

Archivos nuevos: `src/rasterization/{mode,coverage,barycentric,scanline}.rs`,
`src/benchmark/{mod,triangles}.rs`, `src/geometry/demo.rs`. Archivos
modificados: `src/pipeline/rasterizer.rs` (despacho), `src/pipeline/render.rs`
(`render_with_mode`; `render` sigue siendo el wrapper wireframe), `src/app.rs`
(CLI). `Mesh`, el parser OBJ, `line`, `triangle`, `primitive_assembly`,
`vertex_shader`, `fragment_shader` y `Framebuffer` no cambiaron.

## Convención de cobertura compartida (`coverage.rs`)

Espacio de pantalla: `x` a la derecha, `y` hacia abajo; el centro del píxel
`(x, y)` es `P = (x + 0.5, y + 0.5)`. Aritmética en `f64`.

```text
E(A, B, P) = (Px - Ax) * (By - Ay) - (Py - Ay) * (Bx - Ax)
area2      = E(A, B, C)
```

- Si `area2 < 0` se intercambian B y C localmente (la malla no se toca); si
  `area2 == 0` el triángulo es degenerado y no produce interior.
- Regla **top-left** para empates (`E == 0`): la arista `A→B` acepta el punto
  si `By > Ay` (arista que baja) o si `By == Ay` y `Bx < Ax` (horizontal hacia
  la izquierda). Un punto está cubierto si las tres aristas lo aceptan.
- Consecuencia probada: dos triángulos que comparten una arista cubren cada
  centro de esa arista exactamente una vez (sin huecos ni dobles escrituras).
- `center_range(min, max, limit)` convierte un intervalo flotante en índices de
  píxel cuyos centros caen dentro, con `ceil(min - 0.5)` y `floor(max - 0.5)`,
  recortado a `[0, limit)` y sin desbordes de conversión.

## Algoritmo 1: `fill_barycentric`

1. Orientar el triángulo y descartar área cero o coordenadas no finitas.
2. Calcular el bounding box y recortarlo al framebuffer (`center_range`).
3. Para cada centro `P` del rectángulo calcular los tres pesos
   `w_a = E(B,C,P)/area2`, `w_b = E(C,A,P)/area2`, `w_c = E(A,B,P)/area2`.
4. Emitir un fragmento si los tres numeradores pasan la regla top-left.
5. Color uniforme del primer vértice, profundidad 0 (como el wireframe).

## Algoritmo 2: `fill_scanline`

1. Orientar el triángulo igual que arriba y calcular el rango de filas.
2. Por cada fila `y` con línea de muestreo `sy = y + 0.5`, intersectar solo
   las aristas no horizontales con `sy ∈ [min_y, max_y)` (semiabierto): un
   vértice compartido por dos aristas se cuenta una vez. `x = x0 + t·(x1-x0)`.
3. Una arista horizontal exactamente sobre `sy` decide la fila completa con la
   regla top-left (se acepta o se rechaza entera).
4. El span `[xl, xr]` se convierte a índices de píxel; los píxeles interiores
   se emiten directamente y solo los dos extremos de cada lado se clasifican
   con `E` para resolver empates y redondeo. Nunca se recorre el bounding box
   completo ni se llama a la función baricéntrica (`tests/lab5_final_contract_tests.rs`
   lo verifica leyendo el código fuente).

## Comparación de posiciones

Para cada caso se extraen los pares `(x, y)` de ambos algoritmos, se comprueba
que no hay duplicados dentro de un mismo triángulo, que todo está dentro del
framebuffer y que `set_barycentric == set_scanline`. Un fallo imprime los
primeros píxeles de `only_in_barycentric` y `only_in_scanline`.

| Caso | A | B | C | Resultado |
|---|---|---|---|---|
| T01 pequeño | (10,10) | (18,12) | (12,20) | igual |
| T02 grande | (10,10) | (90,20) | (40,85) | igual |
| T03 borde horizontal | (10,20) | (70,20) | (40,75) | igual |
| T04 borde vertical | (20,10) | (20,80) | (75,40) | igual |
| T05 winding invertido | (10,10) | (40,85) | (90,20) | igual a T02 |
| T06 recorte parcial | (-15,20) | (50,15) | (40,115) | igual, todo dentro |
| T07 colineal | (10,10) | (30,30) | (50,50) | vacío en ambos |
| T08 vértice repetido | (20,20) | (20,20) | (60,50) | vacío en ambos |
| T09 muestras sobre aristas | (10.5,10.5) | (60.5,10.5) | (10.5,60.5) | igual, 1275 px |

Estrés (`tests/triangle_edge_cases_tests.rs`): 300 triángulos pseudoaleatorios
con LCG de semilla `0x5EED1AB5` en cuatro viewports (coordenadas a cuartos de
píxel, con vértices fuera del área), las seis permutaciones de cuatro
triángulos, triángulos minúsculos, recorte por cada borde, cobertura total con
coordenadas de 1e7, viewports vacíos, NaN/inf y 200 triángulos en un framebuffer
de 1×1. Sin discrepancias.

Con la nave (708 caras en vista X/Z): wireframe 79 934 fragmentos; ambos
rellenos 173 500 fragmentos con conjuntos idénticos (`tests/lab5_scene_tests.rs`,
`tests/lab5_final_contract_tests.rs`).

## Ejecución

```bash
cargo run --release -- --scene triangles --mode wireframe
cargo run --release -- --scene triangles --mode barycentric
cargo run --release -- --scene triangles --mode scanline
cargo run --release -- --mode scanline                      # nave del Lab 4 rellena
cargo run --release -- --scene triangles --mode scanline --screenshot captura.png
cargo run --release -- --benchmark-triangles                # sin ventana, escribe CSV
cargo run --release -- --benchmark-triangles --benchmark-output /otro/dir
cargo run --release -- --benchmark-triangles --profile --profile-mode scanline --profile-iterations 1000
```

`--mode` por defecto es `wireframe` y `--scene` por defecto es `ship`, así que
`cargo run` sigue siendo exactamente el Laboratorio 4.

## Benchmark

Metodología (`src/benchmark/triangles.rs`):

- Carga fija: la escena de demostración (7) más 200 triángulos sembrados
  (`0x1AB50005`), 207 en total, en 800×600, sin degenerados ni totalmente fuera
  del viewport. Se construye antes del cronómetro.
- Antes de medir se verifica que ambos rellenos cubren los mismos píxeles por
  triángulo; el total es 858 033 fragmentos por iteración para los dos.
- Tramo cronometrado: `Instant::now()` … `elapsed()` alrededor de 100
  llamadas a `rasterize_with_mode` (crear y almacenar los `Vec<Fragment>`), con
  `black_box` en entrada y salida. Sin ventana, sin textura, sin framebuffer,
  sin impresión.
- 10 rondas de calentamiento descartadas; 7 rondas medidas por modo,
  alternando modos en cada ronda; mediana de las 7 duraciones.

Entorno de la medición registrada: Intel Core i7-13650HX, Fedora 43,
rustc 1.97.0, `cargo run --release`, 2026-10-21 22:52 (CST). Datos crudos en
`docs/results/lab5/benchmark_raw.csv`; resumen derivado de esos mismos datos en
`docs/results/lab5/benchmark_summary.csv`.

| Modo | Triángulos | Fragmentos/iteración | Rondas × iteraciones | Mediana por ronda | Mediana por iteración | Mín–máx por iteración |
|---|---:|---:|---:|---:|---:|---:|
| FilledBarycentric | 207 | 858 033 | 7 × 100 | 1 394 561 562 ns | 13.95 ms | 13.82–14.58 ms |
| FilledScanline | 207 | 858 033 | 7 × 100 | 526 267 211 ns | 5.26 ms | 5.13–6.05 ms |

Scanline fue 2.65× más rápido en esta carga: 6.1 ns por fragmento frente a
16.3 ns. `Wireframe` no se incluye en la tabla porque produce otro trabajo
(solo contornos) y no es comparable.

## Perfilado

`cargo-flamegraph`, `perf`, `valgrind` y `samply` no están instalados y
`samply` (instalado y luego retirado en espacio de usuario) exige
`perf_event_paranoid ≤ 1`, es decir cambios del kernel con sudo que no se
hicieron. Se usó en su lugar un **perfilador por muestreo de pilas** con
`eu-stack` (elfutils, ptrace, sin privilegios):
`docs/profiles/lab5-stack-sampler.sh` lanza
`--benchmark-triangles --profile --profile-mode <modo>` y captura la pila del
hilo principal cada ~10 ms hasta que termina; después agrega conteos *self*
(marco hoja) e *inclusive*. Las muestras crudas están en
`docs/profiles/lab5-stack-samples-*.txt` y los resúmenes en
`docs/profiles/lab5-stack-summary-*.txt`. Un porcentaje es proporción de
muestras, no tiempo absoluto ni orden de ejecución.

| Modo | Muestras | Marco hoja principal | `memcpy` (extend de fragmentos) | Otros |
|---|---:|---|---:|---|
| barycentric (400 iteraciones) | 356 | `fill_barycentric` 87.1 % | 11.8 % | realloc/malloc < 1 % |
| scanline (1000 iteraciones) | 375 | `fill_scanline` 76.5 % | 18.1 % | `ceil` 2.9 %, realloc/free ~2 % |

Interpretación (limitada por el inlining del build release: `edge`, `covers`,
`center_range` y `Fragment::new` aparecen dentro de la función que los llama):

- En baricéntricas casi todo el tiempo es el bucle del bounding box: tres
  evaluaciones de `E` por cada centro candidato, incluidos los que caen fuera.
  Esa es la razón de los 16.3 ns por fragmento.
- En scanline el trabajo por píxel interior es casi nulo (se emiten
  directamente); el costo geométrico se concentra en dos intersecciones por fila
  y en `ceil`/`floor` de `center_range`. Por eso el `memcpy` del `extend`
  (copiar el `Vec` de cada triángulo al acumulado) pesa proporcionalmente más
  (18 %): es el siguiente cuello de botella y sugiere reservar capacidad o
  escribir en un solo `Vec`.
- El crecimiento del `Vec` (`finish_grow`, `grow_one`, `realloc`) es visible
  pero menor al 3 % en ambos.

## Límites

Sin perspectiva, cámara, iluminación, texturas ni z-buffer: dos triángulos
superpuestos se resuelven por orden de dibujo (última escritura gana), como en
el Laboratorio 4. Los colores son uniformes por triángulo; los pesos
baricéntricos se calculan pero todavía no se usan para interpolar atributos.
