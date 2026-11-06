# Reajuste de Crimson Entrance y The King's Crown

Ajuste del Laboratorio 6 sobre dos cuerpos ya existentes. House of Cards no cambia: su código es el mismo y sus cuadros son idénticos bit a bit.

## 1. Cráter-pica (`src/celestial/entrance/events.rs`)

Los tres cráteres circulares y las dos mesetas ♠ independientes se fusionaron en **un único fenómeno geológico**: la corteza se fractura siguiendo una ♠, se hunde en una cuenca con esa forma, su fondo se funde, se enfría y se cierra.

**Altura.** H(n, t) = H_base(n) + Σ A_e(t)·perfil(s_e(n)):
- s_e es la SDF de la pica (`sdf::spade`) en el plano tangente del evento, girada hasta ±30° y erosionada con ruido (±0.04);
- perfil(s) = −(0.85·cuenco + 0.15·fondo) + 0.22·labio, con:
  - cuenco = smoothstep(0.03, −0.13, s), que forma las paredes;
  - fondo = smoothstep(−0.1, −0.24, s);
  - labio = exp(−((s − 0.07)/0.05)²);
- A_e es la profundidad del evento por su envolvente.

La misma altura alimenta la clasificación, el bump y el parallax. La ♠ sale de la depresión, no de una calcomanía.

**Fases** (τ es la fase del ciclo; todo vale 0 en τ = 0 y desde τ = 0.6):

| Fase | τ |
|---|---|
| Fracturas finas que dibujan la ♠ | 0 – 0.08 |
| Hundimiento | 0.05 – 0.2 |
| Profundidad máxima | 0.2 – 0.46 |
| Incandescencia: sube, máximo | 0.22 – 0.34, 0.34 – 0.42 |
| Enfriamiento por parches y costra carbonizada | 0.42 – 0.57 |
| Regeneración: la altura vuelve a la base | 0.46 – 0.6 |
| Reposo y nueva posición | 0.6 – 1 |

El brillo empieza después de la profundidad máxima (0.22 > 0.2). Las máscaras se calculan en una sola pasada por fragmento:
- fracturas: contorno y red, autoiluminadas;
- lava y núcleo casi blanco;
- carbonizado: banda exterior y costra del fondo;
- labio de ceniza;
- anillo ocre de roca alterada.

## 2. Tres ranuras, separación y nuevas posiciones

- **Tiempo.** El evento m nace en m·P/3, con P = `event_period` = 21 s. Su ranura es m mod 3:

  | Ranura | Tamaño (× `event_size` = 0.5) | Profundidad (× `event_depth`) | Brillo máximo |
  |---|---:|---:|---:|
  | Grande | 1.0 | 1.0 | 1.0 |
  | Mediana | 0.72 | 0.85 | 0.9 |
  | Pequeña | 0.5 | 0.7 | 0.8 |

  Cada evento solo se ve en el 60 % de su ciclo, así que hay como máximo dos abiertos y sus máximos nunca coinciden.
- **Separación.** La secuencia se reproduce desde el evento −2. Cada evento prueba hasta 16 candidatos sembrados: los pares en un cono de 55° de la vista inicial, para que la escena de inicio siempre muestre uno, y los impares en 150°. Acepta el primero que deje una holgura ≥ 0 frente a los 4 anteriores (el que sigue abierto y los tres últimos cerrados):

  distancia angular ≥ alcance_a + alcance_b + `event_margin` (0.12 rad)

  El alcance es atan(0.95·tamaño), una cota conservadora de la SDF: radio 0.61, más influencia 0.3, más erosión 0.04.
- **Alternativa segura.** Si ningún candidato sirve, se usa la más despejada de 64 direcciones fijas (Fibonacci).
- **Determinismo.** Cada centro depende solo de su índice: no se mueve mientras se ve, y la misma semilla y el mismo tiempo dan la misma escena. El coste es O(eventos transcurridos) por cuadro, unos 500 por hora simulada.

## 3. Materiales

| Nivel | Material | Paleta | Respuesta |
|---|---|---|---|
| 1 | Lava del cráter-pica (clase `Lava`) | `#4A0B10` → `#FF2020` → `#FF4040`; vetas `#FF6030`/`#FF7A00`; núcleo `#FFF2C4` | Emisiva, igual de día y de noche; el 60 % atraviesa las nubes |
| 2 | Roca volcánica: obsidiana, basalto, pizarra | `#151515`–`#252525`, `#252525`–`#3A3A42`, `#3A3A42`–`#4A4850` | Vítrea (Blinn-Phong), mate, mate |
| 3 | Tierras carmesí | `#4A0B10`–`#8F1018` | Mate, sin emisión |
| 3 | Ceniza y costra | `#6E6468`–`#858080`; costra `#705047`–`#98745B` | *Wrap* 0.3, mate |
| 4 | Fisuras | `#FF7A00`–`#FF9A28` | Emisivas, más brillo de noche |

En el disco visible a t = 8:

| Material | Fracción |
|---|---:|
| Roca oscura | ≈ 75 % |
| Tierras carmesí | ≈ 14 % |
| Ceniza | ≈ 6 % |
| Fisuras | ≈ 3 % |
| Lava | ≤ 2 % |

Cambios respecto a la versión anterior:
- la obsidiana pasó de vidrio rojo a vidrio negro;
- las tierras, de carmesí brillante a vino mate;
- la ceniza, de marfil a gris;
- las fisuras son más finas (`crack_width` = 0.028) y solo aparecen bajo h = 0.02;
- el brillo vítreo es más contenido (`specular_strength` = 0.9, `shininess` = 40).

Las nubes siguen moviéndose solas y proyectando sus sombras. Sobre un cráter abierto su opacidad baja suavemente, como mucho a la mitad (`CRATER_CLEARING`).

**Parámetros nuevos:** `event_period`, `event_size`, `event_depth`, `event_margin` y `lava_glow`; `char_strength` se conserva.

**Retirados:** `crater_period`, `crater_radius`, `crater_depth`, `spade_period`, `spade_size`, `spade_height` y `brand_glow`.

## 4. Rendimiento

**Metodología:**
- `cargo run --release -- --benchmark-shaders`: 800×600, t = 6 s, 1 calentamiento y 5 rondas, mediana por fila, 1 y 20 hilos (i7-13650HX).
- Tres ejecuciones intercaladas de la línea base `4dd51d1` (compilada aparte desde `git archive`) y del código reajustado, con los mismos parámetros por defecto: parallax, bump, nubes y sombras activos.
- Las tablas dan la mediana de las tres. Los datos crudos están en [`results/lab6/crimson_refinement_benchmark.csv`](results/lab6/crimson_refinement_benchmark.csv).
- Los FPS son 1000/(rasterizado + sombreado). No se midieron en la ventana.

**Crimson Entrance**

| Modo | Hilos | Raster (ms) | Sombreado antes (ms) | Sombreado después (ms) | Mejora | FPS antes → después |
|---|---:|---:|---:|---:|---:|---|
| baricéntrico | 1 | 5.6 → 5.6 | 1750.5 | 694.8 | 2.52× (−60 %) | 0.6 → 1.4 |
| baricéntrico | 20 | 6.0 → 5.6 | 251.8 | 62.9 | 4.00× (−75 %) | 3.9 → 14.6 |
| scanline | 1 | 4.2 → 4.2 | 1760.6 | 713.9 | 2.47× (−59 %) | 0.6 → 1.4 |
| scanline | 20 | 4.3 → 4.3 | 237.1 | 61.6 | 3.85× (−74 %) | 4.1 → 15.2 |

**Otros cuerpos**, con el mismo pipeline y ruido compartidos:

| Cuerpo | Modo | 1 hilo antes → después (ms) | 20 hilos antes → después (ms) |
|---|---|---|---|
| The King's Crown | scanline | 289.7 → 270.0 | 55.3 → 25.4 |
| House of Cards | scanline | 119.5 → 127.3 | 25.6 → 12.0 |

**Qué se optimizó.** Las cifras intermedias son aproximadas: se tomaron con 1 hilo y t = 6, con una herramienta temporal de 3 rondas.
1. **Eventos unificados**, de ≈ 1746 a ≈ 1031 ms:
   - las consultas de altura (parallax y bump) solo calculan la altura del evento;
   - una prueba de producto escalar contra el coseno del alcance descarta el resto de la esfera;
   - un evento en reposo no hace nada;
   - las máscaras se evalúan una vez por fragmento.
2. **Bump y parallax**, a ≈ 836 ms:
   - el bump usa diferencias hacia delante desde la altura que ya calculó el suelo: 2 consultas en vez de 4;
   - el parallax usa `parallax_steps`·(1 − 0.65·(N·V)²) pasos, porque en vistas de frente la búsqueda cubre menos terreno.
3. **Ruido**, a ≈ 710 ms, con resultados idénticos bit a bit:
   - tabla de los 12 gradientes en lugar de un `match`, que fallaba la predicción con *hashes* aleatorios;
   - prefijos del *hash* compartidos entre las 8 esquinas.
4. **Ceros exactos:** se omite el ruido de las fisuras por encima de h = 0.02 y el del borde de la cuenca donde no puede llegar a ceniza.
5. **Hilos:** cola de bloques de 512 fragmentos en lugar de un bloque fijo por hilo. Con 20 hilos pasa de ≈ 85 a ≈ 62 ms. El resultado no depende del número de hilos.

Las nubes (dos máscaras por fragmento: la del píxel y la de su sombra, en puntos distintos) siguen siendo la parte más cara que queda. No se recortaron para no cambiar su aspecto.

## 5. The King's Crown

- **Origen de la estrella literal.** Era `Frame::diadem`: una banda en la latitud 50° con cinco puntas triangulares hacia el polo norte, mezclada en oro con opacidad `crown_strength`.
- **Eliminado:**
  - la función;
  - los parámetros `crown_points`, `crown_base_latitude`, `crown_height` y `crown_strength`;
  - los campos `points` y `base_latitude`;
  - su mezcla en el color y en la vista `materials`.

  No queda ninguna máscara evaluándose.
- **Conservado:**
  - plasma, venas y granulación;
  - manchas que respiran;
  - corona pulsante;
  - eclipse;
  - bucles de plasma;
  - paleta, emisión y determinismo.

## 6. Limitaciones

- Los eventos con índice impar suelen abrirse lejos de la vista inicial; se descubren girando la cámara.
- Con 1 hilo, House of Cards tarda un 6.5 % más (sus cuadros son idénticos): su ruido de baja frecuencia predecía bien el `match` anterior. Con 20 hilos va 2.1× más rápido.
- Las capturas, GIF y videos de `docs/screenshots/lab6` y `docs/videos/lab6`, y [`shader_benchmark.csv`](results/lab6/shader_benchmark.csv), muestran la versión anterior al reajuste. No se regeneraron.
