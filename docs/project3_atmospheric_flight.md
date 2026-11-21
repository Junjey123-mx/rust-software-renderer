# Proyecto 3 — Acople gravitacional, vuelo atmosférico y atmósferas

Este documento empieza el 2026-11-20 con la auditoría del problema, antes de tocar el código, y se completa al final con lo que se hizo.

## 1. Auditoría: qué empuja la nave hacia afuera

El síntoma: al acercarse a un mundo, la nave no se queda cerca de la superficie y la vista tiende a salir hacia afuera. Hay cinco causas en el código, y se suman.

1. **El vuelo local solo sube** (`flight::local_motion`):
   - la distancia al centro se calcula como `(distancia + E/Q).max(suelo + holgura)`;
   - al pasar sobre una montaña, la nave sube para respetar la holgura, pero al bajar la montaña no vuelve a bajar: no hay altura objetivo;
   - cada relieve la empuja un poco más hacia afuera y nunca regresa.
2. **El vuelo espacial choca con una esfera alta** (`SolarScene::collision_shapes`):
   - cerca de un planeta, la nave choca con la esfera del relieve más alto, no con el suelo;
   - en Ace Crystal esa esfera mide 1.46 R, así que la nave se detiene muy por encima de la superficie.
3. **La órbita asistida queda alta** (`capture::CaptureZone::around`):
   - se asienta en `floor + 0.25 R`, donde `floor` ya está por encima del relieve más alto (o del anillo de cartas en House of Cards);
   - `E`/`Q` no pueden bajar de `floor`.
4. **No hay entrada automática a un modo cercano:**
   - la captura exige llegar lento y quedarse 0.25 s;
   - el vuelo local exige pulsar `L` dentro de un rango;
   - una nave que llega rápido no entra a ninguno de los dos, rebota contra la esfera de colisión y la gravedad (a lo más un cuarto del empuje) no la retiene.
5. **La cámara es la del espacio** (`camera::FollowRig`):
   - 0.9 u detrás y 0.22 u arriba en cualquier modo;
   - usa el «arriba» de la nave, que antes de la segunda pasada no se nivelaba con el planeta. La nivelación (`5e99480`) ya lo corrige, pero el encuadre sigue pensado para el espacio.

**Lo que no es causa:**

- el cálculo de la gravedad, que es una inversa del cuadrado ablandada y siempre atrae;
- la proyección tangencial: `local_motion` ya avanza sobre el plano tangente, por arcos de círculo máximo.

## 2. Plan

- **Vuelo atmosférico**, solo en el visor y con el controlador de captura activo, para no cambiar las grabaciones ni las pruebas históricas:
  - mientras dura el vuelo local, una **altura objetivo sobre el suelo** que `E`/`Q` ajustan, entre la holgura y el techo de la atmósfera;
  - un controlador de altura suave (atenuación exponencial, críticamente amortiguado) que lleva la nave a esa altura, hacia arriba y hacia abajo, siguiendo el relieve;
  - la holgura sigue como piso duro;
  - el avance se mantiene sobre el plano tangente (`local_motion`), y la nivelación alinea el «arriba» con la normal y la nariz con la tangente.
- **Entradas y salida:**
  - en vuelo espacial con gravedad, entrar a la atmósfera de un planeta (por debajo de su esfera de colisión más un margen) pasa al vuelo atmosférico sin pedir velocidad baja; la nave baja después con suavidad a la altura de exploración;
  - desde la captura orbital, `Q` en el piso de la órbita o `L` bajan a la atmósfera;
  - `SPACE` (turbo) sale de la atmósfera por la tangente y hacia arriba, como el escape tangencial: bloquea la reentrada hasta alejarse.
- **Cámara atmosférica:** más atrás, más alta y mirando más adelante, con una transición suave entre ese encuadre y el del espacio.
- **Anillo de House of Cards:** decorativo en el vuelo atmosférico, que no tiene colisiones. La nave lo atraviesa y no se rompe el control.
- **Atmósferas visibles:** una pasada en pantalla sobre el búfer de profundidad, con un color por planeta:
  - un halo en el borde del disco, más intenso del lado iluminado;
  - un tinte del borde sobre el disco;
  - dentro de la atmósfera, el cielo se tiñe con su color de día y se oscurece de noche.

## 3. Auditoría del POV al abandonar un planeta

A mitad de la pasada llegó un segundo pedido: «FIX — RESTAURACIÓN AUTOMÁTICA DEL POV TRAS ABANDONAR UN PLANETA». Las respuestas salen del código de `bec7319`, antes de esta pasada.

1. **Cómo fija `FollowRig` la orientación:**
   - no tiene estado;
   - `FollowRig::camera` pone el ojo detrás (`-forward`) y encima (`up`) de la nave, y usa `ship.up` como «arriba» de la cámara;
   - así que la orientación de la cámara es la actitud de la nave.
2. **Qué cambiaba cerca de un planeta:** `level_attitude` alineaba `ship.up` con la normal del planeta:
   - del todo en la captura y en el vuelo local;
   - en parte al acercarse;
   - y también en vuelo libre con gravedad, con un peso que solo se apagaba más allá de la cáscara de suelta más un radio.
3. **Qué persistía al salir:** el giro de la nave sobre su nariz. El escape tangencial deja `up` en la normal del punto de salida, y nada lo devolvía.
4. **¿Se heredaba el `local_up`?** Sí, a través de `ship.up`, que es el «arriba» de la cámara.
5. **¿El alabeo afecta la cámara?** No: `bank` solo gira el dibujo (`drawn_ship`); la física y la cámara usan `ship`.
6. **Por qué no se restauraba:** no había un estado de «saliendo del planeta» ni una vertical de referencia para el espacio. La nivelación solo conocía la vertical del planeta y seguía tirando hacia ella en vuelo libre.
7. **Cómo corregirlo reutilizando la arquitectura:**
   - un estado de cámara de una sola vez (`CameraRecovery`) que arranca en el paso de vuelo local a vuelo espacial;
   - gira `ship.up` sobre la nariz hacia `SPACE_UP` (el «arriba» del inicio del juego) y devuelve el encuadre al rig canónico;
   - no toca la posición, la velocidad ni la nariz;
   - y se retira la nivelación en vuelo libre.

**Medición antes y después.** El mismo escape desde la captura ecuatorial de House of Cards, a 60 FPS, con el controlador del visor (`viewer::step`), en `bec7319` y en el código nuevo. `up·space_up` es el «arriba» de la nave contra la vertical del espacio proyectada sobre la nariz.

| Tiempo desde que se pulsa el turbo | 0.25 s | 0.5 s | 0.75 s | 1.0 s | 3.0 s |
|---|---|---|---|---|---|
| `up·space_up` en `bec7319` | 0.000 | 0.000 | 0.000 | 0.000 | 0.000 |
| `up·space_up` ahora | 0.292 | 0.793 | 0.993 | 1.000 | 1.000 |

La distancia al planeta y la velocidad relativa coinciden cuadro a cuadro entre las dos versiones: la corrección no cambia la trayectoria del escape. Captura 26.

## 4. Estados y transiciones

| Estado | `flight` | `capture.state` | `surface_altitude` | Cámara |
|---|---|---|---|---|
| Vuelo espacial | `Space` | `Free` | `None` | rig Follow |
| Aproximación | `Space` | `Approach` | `None` | rig Follow |
| Captura orbital | `Local` | `Captured` | `None` | rig Follow |
| Vuelo atmosférico | `Local` | `Free` | `Some(altura)` | `ATMOSPHERE_RIG` inclinado al horizonte |
| Escape tangencial | `Space` | `Escape` | `None` | recuperación |

**Transiciones:**

- **Espacio → atmosférico** (`atmosphere_entry`): con gravedad, sin turbo, fuera de una captura y por debajo de `atmosphere_top` de un planeta no bloqueado. Baja con suavidad a 1.5 pisos rasantes.
- **Captura → atmosférico:** `L`, o `Q` en el piso de la órbita. Conserva la altura.
- **Atmosférico → espacio:**
  - `SPACE`: `Escape` con el impulso tangencial;
  - `G`: deriva con la última velocidad;
  - `L`: a mano;
  - `F1`–`F6`: viaje rápido a 2.6 R de otro planeta.

  En los tres primeros el planeta queda bloqueado hasta que la nave sale de su cáscara de suelta.
- **Captura → escape:** `SPACE`, como antes.
- **`CAMERA_RECOVERY`:** es un estado de la cámara (`camera_recovery`), no del vuelo.
  - `note_flight_change` corre antes y después del movimiento de cada cuadro, así que lo arranca una sola vez en cada paso de `Local` a `Space`, venga de donde venga.
  - Volver a `Local` lo cancela.
  - Avanza `dt / 0.9 s` con un perfil *smoothstep* y termina exacto en 1.

## 5. Atmósferas

**Módulo:** `src/solar/atmosphere.rs`, una pasada después de resolver el cuadro y antes de presentarlo.

- **Cáscara:** cada planeta tiene aire desde su esfera base (`R`) hasta `atmosphere_top` (0.3 R sobre su relieve más alto), el mismo límite que usa la entrada al vuelo atmosférico.
- **Densidad:** `exp(-u / 0.35) · (1 - u)`, con `u` la altura dentro de la cáscara (0 en el suelo, 1 arriba).
- **Rayo:** el de cada píxel se sigue con seis muestras entre la entrada a la cáscara y el primer obstáculo del Z-buffer. La profundidad óptica es `0.45 · ∫densidad / espesor`.
- **Luz:** cada muestra se ilumina con la estrella (`smoothstep(-0.25, 0.35, normal·sol)`, con 10 % de brillo propio de noche) y toma el color del aire a su altura.
- **Composición:** `color · exp(-0.5 · profundidad) + aire · ganancia · (1 - exp(-profundidad))`.
- **Lo que sale de esa suma:**
  - el halo, porque los rayos que rozan el limbo cruzan más aire;
  - el borde sobre el disco, que crece hacia el limbo;
  - y, desde dentro, la bruma del suelo lejano y del horizonte.
- **Colores:**

  | Planeta | Aire abajo → arriba | Ganancia |
  |---|---|---|
  | Crimson Entrance | rojo anaranjado → rojo | 0.9 |
  | Clover Haven | cian → azul (húmedo) | 1.0 |
  | Neon Hearts | rosa neón → rojo neón | 1.15 |
  | House of Cards | magenta → morado | 1.0 |
  | Joker | esmeralda → morado (extraño) | 1.0 |
  | Ace Crystal | cian → celeste, el más brillante | 1.3 |

- **Solo escala Grand:** `grand_system` y `grand_with_sets` lo encienden. La escala de proyecto, las grabaciones y los oráculos de cuadro no cambian.
- **Costo:** las filas se reparten entre los núcleos, y cada píxel solo lee su propia profundidad. Mediana de 24 cuadros de vuelo rasante a 800×600 con 20 hilos: Clover Haven pasa de 50 a 70 ms y Ace Crystal de 49 a 64 ms. Desde lejos, la pasada solo recorre el recuadro de cada planeta.

## 6. Recalibración: piso rasante y horizonte en cuadro

**Lo que se midió con el primer vuelo atmosférico.** El piso de `393bdc0` era la holgura del vuelo local (`nave + 0.15 R`, 1.41 u en Clover Haven), y la altura de exploración 1.5 veces eso (2.11 u). Visto desde el ojo, el horizonte de una esfera cae `acos(R / (R + h))` bajo la horizontal: con la cámara encima de la nave, más de 35°. El campo de visión vertical mide 60°, así que su mitad (30°) no llegaba al suelo, y una cámara que miraba la tangente veía el planeta solo en el borde inferior o ni eso. Los cuadros de esa prueba se conservan fuera del repositorio.

**Corrección:**

- **Piso rasante:** 2.5 radios de nave (0.80 u) sobre el suelo, nunca por encima de la holgura del vuelo local. La exploración se asienta a 1.5 pisos (1.20 u), y `local_step` usa ese piso mientras corre el controlador de altura.
- **Cámara:**
  - `ATMOSPHERE_RIG` pone el ojo 0.9 u detrás y 0.9 u encima de la nave, así que la nave se ve contra el suelo;
  - `tilt_to_horizon` inclina la vista hasta que el horizonte del planeta más cercano queda 0.12 rad sobre el centro, como mucho 1.2 rad bajo la horizontal y nunca hacia arriba;
  - la inclinación usa el mismo peso que la transición de encuadre, y se deshace con la recuperación.

**Reescalado adicional: no aplicado.**

- **El efecto sería chico:** la caída del horizonte crece como `√(2h/R)`. A la altura de exploración (1.2 u) cae 30.9° en Clover Haven; duplicar los radios de los planetas solo la bajaría a 22.5°.
- **El costo sería grande:** arrastraría las órbitas (que se resuelven con el hueco mínimo), la estrella y sus adornos, los cometas, el `mu` de diseño de cada planeta, las siete pruebas de la escala Grand y las capturas 16–22.
- **Los facetados crecerían:** el nivel de detalle máximo es la icoesfera de nivel 5 (arista ≈ 0.033 R), y su arista crecería con R junto a la nave.
- **Lo que de verdad alejaba el suelo** era el piso de 0.15 R y una cámara que miraba la tangente. Eso se corrigió.

Queda como límite: los mundos se sienten como planetas pequeños que se recorren de cerca, con la curvatura muy marcada, no como llanuras.
