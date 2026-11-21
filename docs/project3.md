# Proyecto 3 — Guía de arquitectura

Cómo está hecho el sistema solar navegable del Proyecto 3: módulos, datos del sistema y fórmulas de escala, órbitas, gravedad, colisión y vuelo. Todo corre en el rasterizador por CPU de este repositorio. raylib solo abre la ventana, lee teclado y ratón y presenta el framebuffer como textura.

## 1. Mapa de módulos

| Módulo | Responsabilidad |
|---|---|
| `src/solar/system/` | Datos validados del sistema: `spec.rs` (estrella, planetas, semillas, adjuntos y `FIFTH_PLANET`), `scale.rs` (proporciones respecto de la nave) y `layout.rs` (separación entre órbitas). |
| `src/solar/scene.rs` | La escena: mallas compartidas, lista plana de cuerpos, nave, cámara y `render_frame`. Arma el cuadro con descarte por esfera envolvente y nivel de detalle. |
| `src/solar/orbit.rs`, `src/solar/clock.rs` | Órbitas circulares en forma cerrada y el único reloj simulado (pausa con `P`). |
| `src/solar/planet/` | Planetas procedurales: icosfera (`icosphere.rs`), campos de altura (`field.rs`, `clover.rs`, `hearts.rs`), mallas por nivel (`geometry.rs`) y materiales (`material.rs`). |
| `src/solar/attach/` | Geometría pegada a un planeta: árboles y rocas (`props.rs`), cascadas, pirámide-corazón, tráfico neón y motos de luz con estelas (`traffic.rs`, `path.rs`). |
| `src/solar/physics/` | Vuelo inercial a paso fijo (`motion.rs`), gravedad (`gravity.rs`) y colisión por barrido (`collision.rs`). |
| `src/solar/flight.rs`, `src/solar/ship.rs`, `src/solar/thrusters.rs` | Controladores de vuelo espacial y local, estado de la nave y llamas de los motores. |
| `src/solar/capture.rs` | Captura orbital con histéresis y escape tangencial, sobre los controladores de vuelo local y espacial. |
| `src/solar/nebula.rs`, `src/solar/comets.rs`, `src/solar/ship_lights.rs`, `src/solar/audio.rs` | Cielo que cicla de púrpura a azul, cometas, luces automáticas de la nave y audio procedural. |
| `src/solar/camera.rs`, `src/solar/viewer.rs`, `src/solar/input.rs`, `src/solar/hud.rs` | Cámaras Follow y Orbit, bucle de ventana, teclas y contadores en pantalla. |
| `src/solar/record.rs` | Grabación sin ventana de los guiones `flight`, `tour` y `final-tour` (cuadros RGB crudos por `stdout`). |
| `src/benchmark/solar.rs` | Escenarios medibles (A–M) y el benchmark final de C75. |

## 2. El sistema

La nave es la unidad de medida. Su radio envolvente es `MESH_RADIUS × SHIP_SCALE ≈ 0.319` unidades de mundo.

| Cuerpo | Radio R (u) | R / nave | Radio de órbita (u) | Velocidad lineal (u/s) | Periodo (s) | `mu` (u³/s²) |
|---|---:|---:|---:|---:|---:|---:|
| The King's Crown (estrella) | 5.40 | — | 0 | — | — | sin gravedad |
| Crimson Entrance | 1.10 | 3.45 | 14.0 | 1.00 | 88.0 | 0.994 |
| Clover Haven (trébol) | 1.45 | 4.55 | 21.0 | 0.90 | 146.6 | 1.631 |
| Neon Hearts | 1.30 | 4.08 | 29.5 | 0.85 | 218.1 | 1.388 |
| House of Cards | 1.75 | 5.49 | 40.0 | 0.75 | 335.1 | 1.969 |
| Joker | 1.60 | 5.01 | 49.0 | 0.70 | 439.8 | 1.800 |

- **Planetas activos:** cinco y la estrella. El Joker, quinto planeta, quedó aprobado y activo el 2026-11-20 ([decisión](project3_fifth_planet_decision.md)).
- **Ace Crystal:** su diseño también está aprobado, pero no está implementado (sección 6).
- **Reescalado:** la evaluación de un reescalado mayor está en [project3_rescale_evaluation.md](project3_rescale_evaluation.md).

## 3. Fórmulas

### Escala

- **Radio de mundo:** `R = escala × radio_malla`. La escala de un cuerpo es `R / SPHERE_RADIUS` (`ScaleProfile::world_scale_for`).
- **Planetas:** `3 ≤ R / R_nave ≤ 6`.
- **Estrella:** `3 ≤ R_estrella / R_planeta_mayor ≤ 5`; aquí es 5.40 / 1.75 = 3.09.
- **Tolerancia:** relativa de `1e-3` (`RATIO_TOLERANCE`).
- **Separación:** las cáscaras efectivas de cuerpos vecinos quedan al menos a 3 u.
- **Velocidades:** ninguna órbita supera 1 u/s de velocidad lineal, y la nave crucera a 3 u/s, así que siempre alcanza un planeta sin viaje rápido.

### Órbitas (`src/solar/orbit.rs`)

```
ángulo(t)   = fase + ω t,         ω = v / r     (v lineal, r radio de órbita)
posición(t) = centro + r (cos ángulo, 0, sin ángulo)
periodo     = 2π / |ω| = 2π r / v
```

La posición depende solo del tiempo simulado: misma `t`, mismo cuadro, a cualquier FPS. Cada planeta además gira sobre su eje (`spin`, rad/s), y lo que tiene adjunto (props, tráfico, estelas) se transforma con él.

### Gravedad (`src/solar/physics/gravity.rs`)

```
a(p) = Σ_i  mu_i (c_i − p) / (|c_i − p|² + ε_i²)^(3/2),     ε_i = 0.25 R_i
```

- **Ablandamiento:** es una inversa del cuadrado ablandada. En el centro la aceleración es 0, su máximo `2 mu / (3√3 ε²)` está a `ε/√2`, y lejos coincide con `mu / d²`.
- **Superposición:** la atracción de varios planetas se suma.
- **Sin estrella:** la estrella no atrae, y los cuerpos siguen sus órbitas sin n-cuerpos.
- **`mu` de diseño:**

  ```
  mu = min( 0.25 · THRUST_ACCEL · 1.0625^(3/2) · R²,  (0.5 · v_crucero)² · R / 2 )
  ```

  Con `THRUST_ACCEL = 3` y `v_crucero = 3`, la atracción en la superficie no pasa de un cuarto del empuje y la velocidad de escape no pasa de la mitad de la de crucero: la nave siempre puede escapar.
- **Activación:** `G` la enciende o apaga, y el HUD muestra `GRAV ON/OFF`.
- **Evidencia:** [gravity_evidence.md](results/project3/gravity_evidence.md) y [05-gravity-on-off.png](screenshots/project3/05-gravity-on-off.png).

### Integración y colisión

- **Paso fijo:** la física avanza a `1/240 s` (`PHYSICS_STEP`), con hasta 24 subpasos por cuadro, de modo que la trayectoria es la misma a 30, 60 o 144 FPS.
- **Vuelo espacial:**
  - el empuje acelera a 3 u/s²;
  - `SPACE` lleva la velocidad a 8× en 0.4 s y la baja en 0.6 s;
  - sin empuje, los amortiguadores (`X`) reducen la velocidad por `exp(−1.5 dt)`, a un quinto en cerca de un segundo;
  - el giro del ratón se suaviza con `TURN_EASE = 12`.
- **Colisión:**
  - cada subpaso barre el segmento de la nave contra la esfera `R·(1 + altura_máx) + R_nave + 0.05`, en el marco móvil del cuerpo;
  - al tocar, la nave queda sobre esa esfera y conserva solo la velocidad tangencial;
  - en vuelo local, el suelo es el campo de altura del planeta.
- **Planetas en movimiento:** el reloj coloca los planetas al final del cuadro antes del vuelo. Por eso cada forma de colisión arranca donde estaba su planeta al **inicio** del cuadro (`posición − v_planeta·dt`) y avanza con él en cada subpaso. La nave se compara así siempre con el planeta en el mismo instante, con su velocidad relativa.
- **Corrección del 2026-11-20:** hasta entonces la forma arrancaba en la posición final y adelantaba al planeta un cuadro de su órbita (0.035 u dentro del trébol a 10 FPS). Con la corrección, la nave conserva el margen de 0.05 con los cinco planetas a 10, 15, 30, 60 y 144 FPS (`tests/p3_collision_moving_target_tests.rs`).

### Captura orbital y escape tangencial (`src/solar/capture.rs`)

Activa en el visor interactivo (`cargo run --release -- --scene project3`). Las grabaciones y las pruebas anteriores vuelan igual que antes (`OrbitCapture::enabled` queda apagado).

- **Estados:** vuelo espacial → aproximación gravitatoria → captura orbital → escape tangencial. `L` desde la captura pasa a exploración local. Hay un solo estado a la vez, y el título de la ventana y el HUD muestran `ORBITAL CAPTURE - <planeta>`.
- **Cáscaras por planeta (`CaptureZone`):**

  ```
  piso     = max(R·(1 + altura_máx) + holgura,  R·extensión_anillo + R_nave + 0.1 R)
  órbita   = piso + 0.25 R
  captura  = órbita + 0.35 R
  suelta   = captura + 0.5 R
  ```

  El anillo solo existe en House of Cards (2.13 R). Allí el piso queda por encima de todas las cartas: la órbita asistida nunca cruza el anillo, ni siquiera con `Q`.
- **Entrar:** la gravedad debe estar encendida (`G`). La nave tiene que estar entre el piso y la cáscara de captura, a no más de 1.25 × la velocidad de crucero respecto del planeta y sin alejarse, durante 0.25 s. A velocidad de la luz atraviesa la cáscara sin quedar capturada.
- **Mantenerse:** es el vuelo local con un empuje automático de 0.6 a lo largo de la nariz y una altura objetivo.
  - la nave se acerca a esa altura con constante de tiempo de 0.5 s;
  - `E`/`Q` mueven el objetivo dentro de la banda;
  - `W`/`S`, `A`/`D` y el ratón siguen funcionando;
  - el planeta se mueve y la órbita lo acompaña.
- **Escapar (`SPACE`):**
  - la nave sale por la tangente de su órbita con la velocidad del planeta más `max(0.6 · v_local, 1.1 · √(2 mu / d))`, es decir, al menos 1.1 veces la velocidad de escape de esa altura;
  - mientras se mantiene `SPACE`, empuja a fondo con velocidad de la luz, hasta 3 s o hasta salir de la cáscara de suelta;
  - la distancia al planeta solo crece.
- **Sin recaptura inmediata:** el planeta que soltó la nave no puede volver a capturarla hasta que salga de la cáscara de suelta. Esa es la histéresis entre `captura` y `suelta`.
- **Viaje rápido, `G` y `L`:**
  - el viaje rápido (F1–F5) termina la captura;
  - `G` apagada la termina donde está la nave;
  - `L` la cambia por exploración local, sin deriva.
- **Pruebas:** `tests/p3_orbital_capture_tests.rs` cubre los cinco planetas, una vuelta completa sin teclas, el anillo, el escape, la recaptura, 10/30/60/144 FPS, el cambio de cámara, el viaje rápido, `G` y `L`.

### Nivel de detalle y descarte

- **Mallas de relieve:** cada planeta tiene icosferas de niveles 2, 3, 4 y 5 (`20·4ⁿ` triángulos: 320, 1280, 5120 y 20480), deformadas por su campo de altura.
- **Elección de nivel:** el nivel depende del radio proyectado en píxeles. Sube al pasar 48, 140 y 360 px y baja bajo 40, 120 y 300 px; la histéresis evita parpadeos.
- **Descarte por esfera envolvente:** todo objeto fuera del frustum se descarta antes de rasterizar (`C` lo apaga para comparar).
- **Contadores:** el HUD (`H`) muestra objetos probados y descartados, triángulos y nivel por planeta ([06-hud-counters.png](screenshots/project3/06-hud-counters.png)).
- **Benchmark:** las cifras medidas están en [benchmark_final.md](results/project3/benchmark_final.md).

## 4. Ambiente de la expansión (2026-11-20)

Solo el sistema del Proyecto 3 activa estas mejoras. Las escenas de los laboratorios y del camino al Proyecto 3 no cambian.

- **Cielo (`nebula.rs`):**
  - la nebulosa pintada se recolorea en su rejilla de muestreo con una fracción de azul `drift · ciclo · onda`;
  - el ciclo es `0.5 − 0.5 cos(2π t / 90)` y la onda es `0.55 + 0.45 sin(2.5 d·eje − 2π t / 37)`;
  - de púrpura a azul y de vuelta en 90 s, sin que todo el cielo cambie a la vez;
  - prueba: `p3_sky_cycle_tests`.
- **Cometas (`comets.rs`):**
  - tres elipses de forma cerrada alrededor de la estrella;
  - la cola mide 0.75 desde la estrella más 0.25 hacia atrás;
  - se dibujan después del Z-buffer, detrás de lo que ya está pintado, y se omiten fuera del cuadro;
  - prueba: `p3_comet_tests`.
- **Agua y cascadas (`planet/clover.rs`, `attach/waterfalls.rs`):**
  - el océano suma al tren de olas y a la espuma un oleaje de bandas curvadas que cruza el mar cuatro veces por periodo de olas;
  - a las cuatro cascadas clásicas se suman tres monumentales: 0.06 R de ancho y 24 segmentos;
  - cada monumental nace al 40 % del camino entre el centro de su montaña y la costa, en otra ladera, a al menos 0.12 rad de la clásica, y baja por el gradiente hasta el mar;
  - prueba: `p3_monumental_waterfall_tests`.
- **Luces de la nave (`ship_lights.rs`):**
  - el nivel es la profundidad de la nave en la sombra de algún planeta entre ella y la estrella (penumbra de 1.15 R a 0.85 R del eje);
  - se dibujan las luces roja y verde de las alas, un estrobo de 0.12 s por segundo y un haz delantero;
  - al sol no se dibuja nada: el cuadro queda idéntico byte a byte;
  - prueba: `p3_ship_lights_tests`.
- **Audio (`audio.rs`):**
  - al iniciar se sintetizan tres sonidos PCM de 16 bits a 22 050 Hz:
    - el rumor de los propulsores: un zumbido de 55 Hz sobre parciales graves;
    - el silbido de la luz: parciales agudos;
    - un barrido de 180 a 900 Hz en 0.9 s;
  - los dos bucles duran un segundo exacto con ciclos enteros, así que no hacen clic;
  - el visor los carga en el dispositivo de audio de raylib, el volumen sigue al empuje y al turbo, y `M` silencia;
  - prueba: `p3_audio_tests`.

## 5. Recorrido grabado (histórico)

Es el recorrido de la versión de cuatro planetas, conservado como evidencia. El video definitivo de la entrega lo graba el autor (README, sección «Video de demostración»).

- **Guion:** `--timeline final-tour` (`src/solar/record.rs`), un solo plano de 56 s:
  - vuelo espacial y acercamiento a Crimson Entrance;
  - una vuelta completa en vuelo local;
  - pasadas junto al trébol y a Hearts;
  - salida a velocidad luz.
- **Pilotaje:** la nave se pilotea con las mismas acciones que un jugador (teclas de vuelo y giros de ratón), con la gravedad encendida.
- **Sin cortes:** no hay viaje rápido ni cambio de cámara.
- **Pruebas:** `tests/p3_final_tour_tests.rs` comprueba que no hay cortes, que la vuelta supera 2π, que el camino es continuo a 30 y 60 FPS sin entrar en ningún cuerpo y que dura a lo sumo 60 s.
- **Regenerar:**

  ```bash
  cargo build --release
  docs/videos/project3/encode.sh          # 01-final-tour.mp4 y 01-final-tour.gif
  ```

## 6. Verificación

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --release
```

- **Oráculos de cuadro:** los oráculos de los laboratorios 4, 5 y 6 y los de escena (`LEGACY_A_J`, `P3_SCALED_A_J`, `P3_CLOVER_K`, `P3_HEARTS_M`) fijan los cuadros bit a bit.
- **Rubros:** la matriz [project3_rubric_claims.md](project3_rubric_claims.md) dice qué rubro se reclama, con qué suite y con qué captura.
- **Requisitos habilitantes:** el contrato R1–R5 está en [project3_delivery_contract.md](project3_delivery_contract.md).
