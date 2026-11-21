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
