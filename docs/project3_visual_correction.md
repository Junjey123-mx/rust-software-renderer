# Proyecto 3 — Segunda pasada: identidad visual, escala y navegación

Este documento empieza el 2026-11-20 con la auditoría y el plan, antes de tocar el código, y se completa al final con lo que se hizo. Cada afirmación de la auditoría sale de leer el código o de una medición ya registrada.

## 1. Auditoría previa

### 1.1 Clover Haven no comunica su identidad acuática

- **No es falta de agua:** el fondo marino (`seabed`, de −0.032 a 0.008 R) queda siempre bajo el nivel del mar (0.02 R). El mar ya cubre todo menos los cuatro tréboles.
- **El problema es el color y la luz** (`planet/clover.rs`):
  - el agua profunda es azul marino oscuro (`DEEP = #0A2F66`) y la poco profunda, un turquesa estrecho (`SHALLOW`, hasta 0.03 R de profundidad);
  - en la cara de noche solo queda un 3.5 % de luz ambiente;
  - desde lejos, el mar se lee como espacio oscuro y los tréboles verdes son lo único que destaca.
- **El movimiento es débil:**
  - el oleaje (`swell`) es un brillo de cielo del 14 %, y la espuma solo aparece en una franja de 0.02 rad de la costa;
  - no hay corrientes ni destellos que se lean a distancia.

### 1.2 Las cascadas no se ven

- **Cómo son:** cada cascada (`attach/waterfalls.rs`) es una cinta pegada al terreno, a 0.004 R sobre la ladera, que sigue el descenso hasta el mar.
  - Las clásicas miden 0.02 R de ancho y las monumentales, 0.06 R.
  - Sobre montañas de 0.12 R de alto con laderas suaves, se leen como arroyos pintados, no como agua que cae.
- **Medido en el benchmark de la expansión** (`docs/results/project3/benchmark_expansion.md`):
  - apuntando a una monumental desde 0.3 R, cambia 663 píxeles de 480 000;
  - en los encuadres K, K2 y K3, 0.
- **Se ocultan de lejos:** se dejan de dibujar con los adornos mientras el planeta mide menos de `PROP_MIN_PX` en pantalla.

### 1.3 Ace Crystal no se distingue porque no existe

- **Estado real:** Ace Crystal no está en la escena. El 2026-11-20 se aprobó y se decidió no implementarlo (`docs/project3_fifth_planet_decision.md`). Ningún cuadro del visor lo dibuja.

### 1.4 El reescalado no se aplicó

- **Qué se hizo:** se evaluó y se dejó documentado (`docs/project3_rescale_evaluation.md`), pero `ScaleProfile::project3` sigue en las proporciones originales:
  - planetas de 3 a 6 radios de nave (3.45–5.48 reales);
  - estrella de 3 a 5 radios del planeta mayor (5.40 u frente a 1.75 u).
- **Consecuencia:** el visor, el recorrido grabado y las pruebas usan esa misma escala. Junto a una nave de 0.32 u, un planeta de 1.1–1.75 u parece una pelota pequeña.
- **Valores absolutos** que el reescalado debe recalibrar a mano:
  - la cámara de seguimiento (`FollowRig`: 0.9 u detrás, 0.22 u arriba);
  - la aceleración (`THRUST_ACCEL` = 3 u/s²);
  - las órbitas de los cometas (85–95 u);
  - el punto de aparición.
- **Lo que ya es relativo:**
  - el resto de la captura, el vuelo local, la colisión, los adornos, el LOD y el descarte se miden en radios del cuerpo o en píxeles proyectados;
  - el umbral de captura se mide en múltiplos de la velocidad de crucero.

### 1.5 La nave se ve recta en la captura orbital

- **El vuelo local y la captura nunca alinean la nave con el planeta** (`flight::local_motion`): solo la transportan.
  - Al avanzar, la nave gira con el arco recorrido, pero conserva la actitud con la que entró.
  - Si llega con el «arriba» del mundo y orbita por el ecuador, su «arriba» queda perpendicular a la vertical del planeta durante toda la órbita.
- **La captura** (`capture::captured_step`) reutiliza ese mismo paso: corrige la altura, no la actitud.
- **No hay alabeo:** `A`/`D` desplazan la nave de lado (strafe), y ni ellas ni el ratón la inclinan. La nave cambia de dirección sin cambiar de actitud.

### 1.6 Las luces automáticas no resuelven las zonas oscuras

- **Se activan bien:** `ship_lights::shadow` detecta la sombra del planeta y en los vuelos rasantes del benchmark da 1.0.
- **El problema es qué dibujan:**
  - tres puntos y un haz de 24 brillos, pintados después de resolver el cuadro (unos 3000 píxeles);
  - no iluminan nada: el terreno se sombrea solo con la luz de la estrella, y el relieve en la cara de noche queda negro aunque el haz esté encendido.

### 1.7 Al Joker le falta identidad

- **Lo que tiene** (`planet/joker.rs`): un arlequín de rombos verde esmeralda y morado con crestas negras, pulsos magenta y un glitch suave, sobre un relieve de rombos de poca altura (`JOKER_HEIGHT`).
- **Lo que le falta:**
  - no tiene costuras de neón, motivos de comodín ni asimetrías;
  - la paleta casi no usa el rojo corinto;
  - desde lejos se lee como una esfera verde y morada con bultos.

### 1.8 Qué tocar y qué no

- **Tocar:**
  - `solar/system/{scale,spec}.rs`: una escala nueva para el visor en vivo;
  - `solar/scene.rs` y `solar/viewer.rs`: construir con la escala nueva, alabeo, luces;
  - `solar/capture.rs`: alineación con la gravedad;
  - `solar/ship_lights.rs`: luz sobre el terreno;
  - `solar/planet/{clover,joker}.rs` y un módulo nuevo para Ace Crystal;
  - `solar/attach/waterfalls.rs` y `solar/effects.rs`: cascadas que caen;
  - `solar/comets.rs`: escala de los cometas.
- **No tocar:**
  - `src/pipeline/`, `src/rasterization/`, `src/shading/` y `src/celestial/`;
  - las escenas y los oráculos de los laboratorios 4–6 y de `LEGACY_A_J`;
  - los OBJ y las pruebas históricas `tests/solar_*`;
  - el recorrido grabado `01-final-tour`, que es evidencia de la entrega anterior.

## 2. Plan

### 2.1 Escala Grand para el visor en vivo

- **Una escala nueva** (`ScaleProfile::grand`, `SystemSpec::grand`) que usa el visor en vivo:
  - planetas ×5 (17–27 radios de nave) y estrella ×7, dominante;
  - órbitas resueltas con la misma regla de `OrbitLayout`;
  - crucero y aceleración ×2, para que el sistema siga siendo cómodo de cruzar;
  - cometas a la escala del mundo.
- **Lo demás no cambia:** la escala del Proyecto 3 sigue igual para las pruebas, los oráculos, el benchmark y el recorrido grabado, así que esa evidencia no se reescribe.
- **Ace Crystal:** el sexto planeta, solo en la escala Grand. El menú de la rúbrica cuenta cinco planetas como máximo, así que no cambia los puntos.

### 2.2 Luces que iluminan el terreno

- **Una pasada de iluminación en pantalla** sobre el búfer de profundidad, con el foco de la nave:
  - posición y normal reconstruidas desde la profundidad;
  - foco con cono, alcance limitado y atenuación;
  - se suma al color ya resuelto.
- **Se aplica a cualquier superficie** (terreno, ciudad, adornos) sin tocar el shader de cada planeta, y solo mientras la nave está en sombra.

### 2.3 Navegación orgánica

- **Alabeo visual:** inclinación al girar con `A`/`D` o el ratón, suavizada, que vuelve al centro al soltar. No cambia la física.
- **Integración con la gravedad:**
  - dentro de la influencia de un planeta, el «arriba» de la nave se nivela con la normal local (solo alabeo);
  - en la captura, además, la nariz baja al plano tangente;
  - todo por rotación mínima y suavizada, sin saltos ni vuelcos, y la cámara la acompaña.
  - Solo actúa con la captura del visor en vivo activa, así que el vuelo histórico de las pruebas no cambia.

### 2.4 Clover Haven y sus cascadas

- **Océano:**
  - turquesa y cian luminosos, con corrientes animadas, destellos y una costa más legible;
  - espuma al pie de cada cascada.
- **Cascadas que caen:**
  - las tres monumentales pasan a ser chorros que salen del borde alto y caen en parábola hasta el mar, más anchos, de doble cara y con espuma y bruma en la base;
  - siguen visibles a más distancia que los adornos.

### 2.5 Joker y Ace Crystal

- **Joker:** arlequín con costuras de neón, asimetrías, motivos de comodín y la paleta completa (negro, morado, esmeralda, magenta y rojo corinto).
- **Ace Crystal:** cristales tridimensionales grandes (prismas con punta) sobre un suelo facetado, en cian, celeste, turquesa y blanco con reflejos lilas, brillo especular y destellos emisivos.

### 2.6 Orden de los commits

1. Auditoría y plan (este documento).
2. Luces sobre el terreno.
3. Alabeo de la nave.
4. Integración con la gravedad.
5. Escala Grand.
6. Clover Haven: océano y cascadas.
7. Joker.
8. Ace Crystal.
9. README y capturas.
10. Cierre de este documento.

Cada commit pasa `fmt`, `check`, `clippy -D warnings` y `cargo test` antes de confirmarse.

## 3. Resultados (cierre, 2026-11-20)

### 3.1 Commits

| # | Commit | Qué hizo |
|---|---|---|
| 1 | `fb859cb` | Auditoría y plan (secciones 1 y 2) |
| 2 | `114ec84` | El faro ilumina el terreno en la sombra (`ship_lights::light_surfaces`) |
| 3 | `cf5b363` | Alabeo visual de la nave en el visor (`flight::bank_target`, `ease_bank`) |
| 4 | `5e99480` | Nivelación con la vertical del planeta en la captura, el vuelo local y cerca de los planetas (`SolarScene::align_to_planet`) |
| 5 | `d91b0c1` | Escala Grand del visor (`ScaleProfile::grand`, `SystemSpec::grand`, `viewer::build_grand_scene`) |
| 6 | `b8be8dd` | Joker teatral: máscaras, paneles de arlequín, costuras de neón, picos de bufón y *glitch* |
| 7 | `b6ed74b` | Ace Crystal, sexto planeta de la escala Grand, y `F6` |
| 8 | `03596af` | Clover Haven: mar luminoso y cascadas monumentales en cortina |
| 9 | `9b1aebb` | README y capturas 16–22 |
| 10 | este commit | Cierre de este documento |

El orden real difiere del plan (2.6): Joker y Ace se confirmaron antes que Clover Haven, porque sus cambios estuvieron listos antes. Cada commit pasó `fmt --check`, `check --all-targets`, `clippy --all-targets -D warnings`, `cargo test --no-fail-fast` y `git diff --check` antes de confirmarse.

### 3.2 Qué estaba mal y qué se corrigió

| Problema (sección 1) | Corrección | Evidencia |
|---|---|---|
| 1.1 Mar oscuro y quieto | Bajíos turquesa, laguna cian y azul luminoso; línea de rompiente, corrientes que derivan, destellos de sol y brillo tenue de noche | `p3_clover_water_identity_tests` (luminancia del mar iluminado, deriva de las corrientes); capturas 17 y 18 |
| 1.2 Cascadas invisibles | Las tres monumentales salen del borde y caen en parábola hasta el mar, seis veces más anchas que una clásica y abriéndose 1.5 veces, con una cúpula de espuma al pie; las siete son de doble cara y se dibujan desde 16 px de radio del planeta | `p3_clover_water_identity_tests` (cortina sobre la ladera, doble cara, distancia de dibujo); captura 18 |
| 1.3 Ace Crystal no existía | Módulo `planet/ace.rs`: esquirlas facetadas y agujas de cristal 3D en cian, celeste, turquesa, blanco y lila, con su propio shader especular y emisivo; sexto planeta solo en la escala Grand | `p3_ace_crystal_tests` (7), `p3_grand_scale_tests`; captura 19 |
| 1.4 Reescalado no aplicado | Escala Grand para el visor: planetas ×5 (17–27 radios de nave), estrella ×7 (4.3 veces el planeta mayor), órbitas de 78.5 a 225 u, crucero y empuje ×2, cometas ×4, aparición fuera de la última cáscara | `p3_grand_scale_tests` (7); captura 16 |
| 1.5 Nave recta en la captura, sin alabeo | Alabeo de hasta 32° hacia el desplazamiento o el giro, con vuelta suave a nivel; nivelación con la vertical del planeta por rotaciones mínimas y suavizadas | `p3_ship_attitude_tests` (6): nivelada en menos de 6 s, sin saltos por cuadro; captura 22 |
| 1.6 Las luces no iluminaban | Pasada de luz en pantalla desde la profundidad: cono y aura con alcance de 16 u y atenuación, solo en sombra | `p3_ship_lights_tests` (suelo más claro en la sombra, nada fuera del alcance); captura 21 |
| 1.7 Joker genérico | Máscaras de comedia y tragedia, paneles en negro, morado, esmeralda, magenta y burdeos, costuras de neón en zigzag, picos de bufón y banda de *glitch*; relieve de 0.08 a 0.12 | `p3_joker_planet_tests::theatrical_palette_and_neon_seams_show` (el Joker anterior no la pasa); captura 20 |

### 3.3 Validación final

- **Calidad:** `fmt --check`, `check --all-targets`, `clippy --all-targets -D warnings` y `git diff --check` limpios; `cargo test --no-fail-fast`: **1061 pruebas, 0 fallos** (antes de la pasada eran 1033).
- **Compilación:** `cargo build --release` terminó sin errores en 33 s.
- **Visor:** `--scene solar --screenshot` abre la ventana con 7 cuerpos (la estrella y seis planetas) a escala Grand y la ayuda de controles con `F1-F6`.
- **Revisión visual:** la escala, el faro, el alabeo, el Joker, Ace Crystal y Clover Haven se revisaron en cuadros del framebuffer antes de confirmarse, y las capturas 16–22 del README son los cuadros finales. La nivelación con el planeta se verificó con pruebas numéricas (`p3_ship_attitude_tests`), sin captura propia.

### 3.4 Oráculos recapturados

Los oráculos de los laboratorios y `LEGACY_A_J` no cambiaron. En `tests/p3_frame_oracle_tests.rs` se recapturaron, con el motivo en la cabecera:

- E, F y J con la luz del faro sobre el terreno;
- C, D, H e I con el Joker nuevo;
- C, D, H, I, K, K2, K3 y M2 con el mar y las cascadas de Clover Haven.

### 3.5 Incidencias durante la pasada

El equipo se reinició dos veces durante compilaciones pesadas, y en ambas se perdió `/tmp` (tmpfs), donde estaban los parches de Ace Crystal y Clover Haven, los directorios de compilación y las herramientas de captura. El repositorio no se vio afectado. Los dos cambios se reconstruyeron repitiendo, sobre el árbol del repositorio, las mismas ediciones registradas en las transcripciones de su trabajo:

- **Ace Crystal:** pasó sus siete pruebas sin cambios.
- **Clover Haven:** se aflojó un umbral de su prueba de corrientes, porque el último ajuste visual las había adelgazado, y se corrigieron dos avisos de `clippy`.

Después del segundo reinicio, las compilaciones se limitaron a una a la vez con 12 trabajos. Las capturas 16–22 se generaron con el código final.

### 3.6 Límites que quedan

- La escala Grand y Ace Crystal existen solo en el visor interactivo; las pruebas, los oráculos, el benchmark y el recorrido grabado siguen en la escala de proyecto con cinco planetas.
- El alabeo y la nivelación los aplica el controlador del visor; las grabaciones no los usan.
- Los cristales de Ace no tienen colisión propia.
- El costo por cuadro sube a escala Grand: de 13 ms en el inicio a 114 ms en un vuelo rasante de noche, con todos los núcleos a 800×600 (tabla en `docs/results/project3/media_expansion.md`).
- El video final sigue pendiente del autor.
