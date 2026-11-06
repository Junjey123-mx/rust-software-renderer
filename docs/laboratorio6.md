# Laboratorio 6 — Guía técnica de los shaders procedurales

Esta guía acompaña la sección «Laboratorio 6» del [README](../README.md). Explica cómo se extendió el pipeline, las fórmulas de cada cuerpo y de cada efecto, todos los parámetros, las pruebas que respaldan cada criterio y cómo reproducir las evidencias.

## 1. Extensión del pipeline

Los tres cuerpos usan la misma `assets/models/sphere.obj` del docente, sin modificar: 98 vértices únicos, 192 triángulos, 288 aristas únicas y 576 esquinas triangulares. La apariencia sale solo del fragment shader.

```text
sphere.obj ─ load_obj ─ expand_vertices (576 esquinas)
   ↓
vertex_shader ─ project_orbit ─ primitive_assembly        (sin cambios, Lab 4/5)
   ↓
¿modo?
 ├─ wireframe ──────────────► render_with_settings          (ruta Lab 4/5, malla real)
 └─ barycentric / scanline
       ↓ culling de caras traseras (face_normal · ojo ≤ 0)
       ↓ fill_barycentric / fill_scanline                    (sin cambios, Lab 5)
       ↓ interpolation_weights en el centro de cada píxel    (nuevo, coverage.rs)
       ↓ SurfaceSample { posición de objeto, normal, profundidad }
       ↓ PreparedShader::shade en hilos std::thread::scope   (nuevo, shaded.rs)
       ↓ Framebuffer::point_depth                            (Z-buffer compartido)
   ↓
Presenter (textura raylib)  o  --record (rgb24 a stdout, sin ventana)
```

- **Aditivo.** Ningún archivo de prueba existente se modificó. `Fragment`, `Vertex`, `CliArgs`, `RenderSettings`, `Uniforms`, los rellenos, la cámara y el framebuffer no cambiaron. Un oráculo de 21 *hashes* FNV-1a (`tests/lab6_regression_guard_tests.rs`) congela la nave, sus tres modos, la cámara orbital y el Z-buffer.
- **Paralelo y determinista.** Las muestras se reparten en trozos contiguos y se unen en orden, así que la imagen es idéntica con 1 o N hilos (`--threads`).
- **Compatible con el proyecto final.** Los colores pasan por `point_depth`, de modo que la nave y los planetas podrán compartir framebuffer y Z-buffer. `NormalSource::Face` sombrea mallas que no son esferas.

## 2. Interpolación

`interpolation_weights(a, b, c, p)` (`src/rasterization/coverage.rs`) calcula los pesos con funciones de arista sobre el triángulo de pantalla **sin orientar**: λa = E(b, c, p)/E(a, b, c) y equivalentes. Suman 1, valen 1 en su vértice y reproducen exactamente los atributos lineales. Como la cámara orbital es ortográfica (afín), interpolar la posición de objeto en pantalla es exacto.

- **Normal suave:** `normalize(object_position)`, válida porque la esfera está centrada en el origen. Las `vn` del OBJ son planas y no se usan.
- **Sin coordenadas UV:** todo el ruido se muestrea en la dirección unitaria 3D, así que no hay costuras de la cruz de cubo.

La cobertura sigue siendo la del algoritmo elegido. Como la interpolación es la misma función para ambos, las imágenes baricéntrica y scanline son idénticas byte a byte (prueba `filled_modes_are_identical_for_every_body`).

## 3. Caja de herramientas `src/shading/`

| Módulo | Contenido |
|---|---|
| `math.rs` | `hex` (paleta exacta), `saturate`, `smoothstep` (también descendente), `mixf`, `mix3`, `ramp` |
| `noise.rs` | `hash3`, `gradient3` (ruido de gradiente con 12 gradientes de arista, *fade* quíntico), `fbm` (lacunaridad 2, ganancia 0.5, normalizado), `ridged`, `warp` |
| `cycle.rs` | `cycle(t, periodo, fase)` → (k, τ); `envelope(τ, subida, meseta, bajada)`, que vale 0 en τ = 0 y τ = 1 |
| `sdf.rs` | `circle`, `triangle` (exacta), `smooth_min`, `spade` (♠: dos lóbulos, hoja y tallo) y `diamond_l1` (♦: \|x\|/a + \|y\|/b − 1) |
| `sphere.rs` | `tangent_frame`, `east_north`, `local_plane` (gnomónica), `from_local`, `rotate_y`, `rotate_axis`, `angular_distance`, `hash_direction` (uniforme en un casquete) |
| `lighting.rs` | `lambert`, `wrap_lambert`, `blinn_phong`, `fresnel`, `bump_normal`, `parallax_shift`, `shell_exit` |

## 4. The King's Crown (`src/celestial/crown.rs`)

Estrella emisiva: no depende de la luz externa.

1. **Temperatura.** T = saturar(0.85·base + 0.35·venas + granulación), con:
   - base = 0.5 + 0.5·1.6·fbm(warp(rotar_y(n, swirl·t)·escala + (0, flow·t, 0)));
   - venas = 1 − smoothstep(0, ancho, |fbm(w·1.7)|);
   - granulación = fuerza·fbm(n·escala_g + 0.4t).
   La rampa va de vino `#4A0B10` a carmesí `#D21F2B`, naranja `#FF7A00`, oro `#FFC63A` y marfil `#FFF2C4`.
2. **Manchas.** Tres centros sembrados en un cono de 50° alrededor de la vista oblicua, que derivan con `spot_drift_speed`. Su radio respira: r·(0.7 + 0.3·sen(2πt/P + 2.1i)). La umbra lleva T hacia 0.05 y la penumbra lo multiplica por 0.55.
3. **Corona.** borde = 1 − n·V. Su ancho pulsa: w·(1 + amp·(2p − 1)), con p = 0.5 + 0.5·sen(2πt/periodo). Se suma oro y marfil en el limbo.
4. **Eclipse.** Ocupa el último `duration/period` de cada ciclo, con centro en t = periodo − duración/2 (22 s por defecto). Oscurece el núcleo por (1 − profundidad·e·(1 − borde)²) mientras la corona sigue encendida.
5. **Bucles de plasma.** Tres ranuras con periodo `loop_period`·(1 + 0.37·i). En el plano tangente de un pie sembrado (cono de 60°), el bucle es la mitad superior de una elipse de semiejes a y b, girada y deformada por fBm, recorrida de un pie (s = 0) al otro (s = 1). La fase τ controla:
   - concentración (0-0.12): brillo en el pie;
   - nacimiento y expansión (0.12-0.55): el frente `head` avanza de 0 a 1;
   - máximo (0.55-0.7): núcleo casi blanco;
   - retorno (0.7-0.88): adelgaza y la cola `tail` drena hacia el segundo pie;
   - estabilización (0.88-1): nada.

   Todo vale 0 en los extremos del ciclo. Una vaina carmesí oscura separa el filamento del plasma y el halo calienta el plasma vecino. El eclipse también lo oscurece. No sale de la silueta: no hay geometría nueva.

## 5. Crimson Entrance (`src/celestial/entrance/`)

### 5.1 Campo de altura y regiones (`terrain.rs`)

- **Altura macro:**
  - cuenca monumental: `crater_profile(d/radio, profundidad)`, con cuenco −p·(1 − d²) y borde +0.35·p·exp(−((d − 1)/0.18)²);
  - colinas: 0.6·fbm(n·escala, 2);
  - cráteres-pica activos (solo su altura).
- **Altura total:** macro + `detail_amplitude`·fbm(3·n·escala, octavas).
- **Regiones,** en orden de prioridad:

  | Condición | Región |
  |---|---|
  | lava > 0.3 | lava del cráter-pica |
  | grieta > 0.5 | fisura incandescente |
  | borde > 0.25 | ceniza |
  | h < −0.12 | obsidiana |
  | h < 0.08 | basalto |
  | h < 0.2 | tierras carmesí |
  | resto | pizarra |

  Cada región mezcla dos tonos por ruido. La roca negra y gris domina, las tierras carmesí son mate y la lava se reserva para los cráteres-pica. Una costra ocre marca las transiciones; las paletas están en [lab6-crimson-refinement.md](lab6-crimson-refinement.md).

### 5.2 Eventos (`events.rs`)

Un solo fenómeno, el **cráter-pica**, con tres ranuras: grande, mediana y pequeña. El evento m nace en m·P/3, con P = `event_period` = 21 s, y ocupa la ranura m mod 3.

- **Altura.** h = A(t)·perfil(s), donde s es la SDF de la pica erosionada en el plano tangente del evento y perfil(s) = −(0.85·smoothstep(0.03, −0.13, s) + 0.15·smoothstep(−0.1, −0.24, s)) + 0.22·exp(−((s − 0.07)/0.05)²). Es la misma altura del parallax, del bump y de la clasificación.
- **Fases por τ:**
  - fracturas que dibujan la ♠ (0–0.08);
  - hundimiento (0.05–0.2);
  - profundidad máxima (hasta 0.46);
  - incandescencia, solo después de la profundidad máxima: sube de 0.22 a 0.34 y llega al máximo entre 0.34 y 0.42;
  - enfriamiento por parches (0.42–0.57);
  - regeneración (0.46–0.6);
  - reposo (0.6–1).

  Todo vale 0 en τ = 0 y desde τ = 0.6, así que el traslado no se ve.
- **Posición.** Cada evento prueba hasta 16 candidatos y acepta el primero cuyo alcance (atan(0.95·tamaño)) más `event_margin` no toque al cráter abierto ni a los tres últimos cerrados. Si ninguno sirve, toma la más despejada de 64 direcciones fijas. Los eventos pares se abren delante de la cámara inicial (cono de 55°) y los impares en cualquier parte (150°). La secuencia se reproduce desde un evento fijo, así que cada centro depende solo de su índice.
- **Emisión.** La lava (`lava_emission`: `#FF2020` → `#FF4040`, vetas `#FF6030`/`#FF7A00`, núcleo `#FFF2C4`) es independiente de la luz y el 60 % atraviesa las nubes. Sobre un cráter abierto la opacidad de las nubes baja hasta la mitad.

Detalles y mediciones en [lab6-crimson-refinement.md](lab6-crimson-refinement.md).

### 5.3 Nubes y sombras (`clouds.rs`)

- **Nubes:** máscara = smoothstep(θ − s, θ + s, 0.5 + 0.8·fbm(warp(rotar_y(n, −ωt)·escala) + (0, deriva, 0))), con θ = 1 − cobertura. Se componen sobre el suelo con su propio *wrap* y la normal geométrica. No hay otra esfera y el terreno no rota.
- **Sombras:** sombra(n, L) = 1 − fuerza·máscara(q)·smoothstep(0, 0.1, μ), con μ = n·L y q = normalizar(n + sL). s = −μ + √(μ² + 2h + h²) es el punto donde el rayo hacia la luz sale de la capa de nubes de radio 1 + h. Solo atenúa la luz directa, nunca la emisión.

### 5.4 Iluminación (`mod.rs`)

- **Parallax:** n_p = `parallax_shift(n, V, altura_macro, escala, pasos, 3)`, con pasos = `parallax_steps`·(1 − 0.65·(N·V)²) y al menos 2. Toda consulta del suelo usa n_p (altura, regiones, albedo, grietas, eventos y bump); la noche, las nubes y las sombras usan n.
- **Bump:** N_b = normalizar(N − k·(h_t·t + h_b·b)). El gradiente de la altura total sale de diferencias hacia delante en el marco tangente, a partir de la altura que el suelo ya calculó (2 consultas).
- **Respuestas:**
  - obsidiana: Blinn-Phong con N_b (tinte cálido `#FFF2C4`);
  - basalto, tierras y pizarra: Lambert mate;
  - ceniza: *wrap* 0.3;
  - fisuras: emisivas (independientes de L).
- **Noche:** factor = smoothstep(0.05, −ancho_terminador, n·L). Emisión = grieta·mezcla(0.12, `night_glow`, factor)·mezcla(`#FF7A00`, `#FF9A28`, grieta). Es un término aparte de la difusa.

## 6. House of Cards (`src/celestial/house.rs`)

- **Vientos diferenciales:** n_w = rotar_y(n, (flow + dw·sen(3πy))·t). Es una rotación, no un ángulo con costura.
- **Bandas:** y' = n_w.y + `band_warp`·fbm(1.6·n_w, 4) y v = 0.5 + 0.5·sen(y'·`band_count`·π/2). La rampa va de `#1A0A20` a `#52106A`, `#8A2BE2`, `#C13CFF`, `#F0408A`, `#F4C6DA` y `#D6D0C4`. Encima se aplican un tinte vino `#4E0E27` y filamentos de fBm sobre (2x, 8y', 2z).
- **Vórtice ♦.** En el plano tangente del centro:
  - d = |u|/a + |v|/b (norma L1);
  - s ∈ [0, 4) es la posición sobre el contorno ♦ (`contour_param`, inversa en `contour_point`);
  - s' = s + W·(4/2π)·caída(d), con caída = (1 − smoothstep(0.6, 1, d))·exp(−2.5d);
  - el punto se reconstruye sobre **el mismo contorno** y las bandas se muestrean en él;
  - fuera de d ≥ 1 es la identidad exacta.
- **Ciclo del vórtice:** periodo de 18 s con calma, perturbación, formación, apogeo (τ ≈ 0.55, t = 9.9), disolución y estabilidad. W = 0 en los bordes del ciclo, y después el centro migra (latitud ≤ 45°). Una tormenta secundaria (periodo de 11 s, escala 0.5) sigue el mismo esquema.
- **Relámpagos neón** (`strike`, `Frame::lightning`). Hay tres canales con ranuras de 0.11, 0.17 y 0.23 s. En cada ranura un *hash* decide si cae un rayo:
  - canal 0: con probabilidad `lightning_rate`·(0.15 + 0.75·envolvente de la tormenta), solo con el vórtice activo; nace dentro del rombo y se anula antes de su borde;
  - canales 1 y 2: con probabilidad `lightning_rate`·0.22, en un casquete del hemisferio visible.

  El destello es exp(−5·fase), a veces con un segundo pulso. El rayo es un trazo dentado (ruido 1D de 4 octavas con los extremos anclados) con dos ramas a ±25-35°, en el plano tangente del punto de impacto. En la tormenta las coordenadas salen del dominio deformado por el vórtice. Lleva un núcleo pálido `#F8E8FF`, un halo `#C13CFF`/`#FF4FB8` y un resplandor tenue en las nubes, todo como emisión.

## 7. Parámetros

Todos se cambian con `--set nombre=valor` (se valida el nombre y el rango). Los interruptores (0 o 1) permiten comparar con y sin cada efecto.

### The King's Crown

| Parámetro | Por defecto | Rango | Cambio visual |
|---|---:|---|---|
| `seed` | 7 | 0 – 9999 | Otra distribución de corrientes y manchas |
| `brightness` | 1 | 0.2 – 2 | Emisión global |
| `plasma_scale` | 1.6 | 0.5 – 8 | Tamaño de las corrientes (mayor = celdas más pequeñas) |
| `plasma_octaves` | 5 | 1 – 7 | Detalle fino de la temperatura (redondeado) |
| `warp_strength` | 0.8 | 0 – 3 | Cuánto se retuercen las corrientes |
| `flow_speed` | 0.18 | 0 – 2 | Advección vertical del plasma |
| `swirl_speed` | 0.25 | 0 – 2 | Giro del dominio deformado (rad/s) |
| `granulation_scale` | 22 | 4 – 60 | Frecuencia de la granulación |
| `granulation_strength` | 0.12 | 0 – 0.5 | Intensidad de la granulación |
| `vein_width` | 0.08 | 0.01 – 0.3 | Grosor de las venas doradas |
| `spot_radius` | 0.32 | 0.05 – 0.8 | Tamaño de las manchas (rad) |
| `spot_breath_period` | 9 | 1 – 60 | Segundos por respiración de una mancha |
| `spot_drift_speed` | 0.05 | 0 – 1 | Deriva de las manchas (rad/s) |
| `corona_width` | 0.45 | 0.02 – 0.6 | Anchura de la corona del limbo |
| `corona_pulse_period` | 4 | 0.5 – 30 | Segundos por pulso de la corona |
| `corona_pulse_amount` | 0.35 | 0 – 1 | Amplitud del pulso (tecla `[` / `]`) |
| `crown_height` | 0.35 | 0 – 0.8 | Altura de las puntas (rad de latitud) |
| `eclipse_period` | 24 | 4 – 120 | Segundos entre eclipses |
| `eclipse_duration` | 4 | 0 – 30 | Duración del eclipse |
| `eclipse_depth` | 0.75 | 0 – 1 | Cuánto se oscurece el núcleo |
| `loops` | 1 | 0 – 1 | Bucles de plasma que brotan y se reabsorben |
| `loop_period` | 9 | 3 – 60 | Segundos por ciclo de un bucle |
| `loop_size` | 0.6 | 0.15 – 0.9 | Semiamplitud del arco en su plano tangente |
| `loop_width` | 0.05 | 0.01 – 0.12 | Grosor del filamento |
| `loop_brightness` | 1.1 | 0 – 3 | Emisión de los bucles |

### Crimson Entrance

| Parámetro | Por defecto | Rango | Cambio visual |
|---|---:|---|---|
| `seed` | 11 | 0 – 9999 | Otra corteza, cuenca y eventos |
| `terrain_scale` | 1.8 | 0.5 – 6 | Frecuencia de las colinas |
| `terrain_octaves` | 5 | 1 – 7 | Detalle del relieve fino (redondeado) |
| `detail_amplitude` | 0.18 | 0 – 0.6 | Rugosidad del relieve fino |
| `basin_depth` | 0.55 | 0 – 1 | Profundidad de la cuenca monumental |
| `basin_radius` | 0.62 | 0.2 – 1.2 | Radio angular de la cuenca (rad) |
| `crack_width` | 0.028 | 0.005 – 0.15 | Grosor de las fisuras |
| `crack_scale` | 7 | 2 – 20 | Densidad de la red de fisuras |
| `events` | 1 | 0 – 1 | Cráteres-pica que se hunden, arden y se cierran |
| `event_period` | 21 | 6 – 90 | Segundos por ciclo (ranuras desfasadas un tercio) |
| `event_size` | 0.5 | 0.15 – 0.9 | Tamaño del cráter-pica grande en su plano tangente |
| `event_depth` | 0.55 | 0 – 1 | Profundidad del cráter-pica grande |
| `event_margin` | 0.12 | 0 – 0.5 | Separación angular extra entre cráteres-pica (rad) |
| `lava_glow` | 1 | 0 – 3 | Emisión de la lava del fondo |
| `char_strength` | 0.85 | 0 – 1 | Oscuridad de la roca carbonizada del cráter-pica |
| `clouds` | 1 | 0 – 1 | Capa de nubes (tecla `C`) |
| `cloud_coverage` | 0.45 | 0 – 1 | Fracción del cielo cubierta (tecla `[` / `]`) |
| `cloud_scale` | 2.4 | 0.5 – 8 | Tamaño de las masas de nubes |
| `cloud_speed` | 0.07 | 0 – 1 | Giro de la capa (rad/s) |
| `cloud_evolution` | 0.05 | 0 – 0.5 | Cambio lento de las formas |
| `cloud_softness` | 0.12 | 0.01 – 0.4 | Suavidad de los bordes |
| `cloud_opacity` | 0.85 | 0 – 1 | Opacidad de las nubes densas |
| `cloud_shadows` | 1 | 0 – 1 | Sombras de nubes (tecla `H`) |
| `shadow_strength` | 0.65 | 0 – 1 | Cuánto bloquea la luz una nube densa |
| `cloud_altitude` | 0.07 | 0 – 0.3 | Altura de la capa: desplazamiento de las sombras |
| `bump` | 1 | 0 – 1 | Relieve por bump mapping (tecla `B`) |
| `bump_strength` | 0.8 | 0 – 4 | Inclinación de la normal por el gradiente |
| `bump_epsilon` | 0.004 | 0.0005 – 0.05 | Paso de las diferencias del gradiente (rad) |
| `specular` | 1 | 0 – 1 | Brillo vítreo de la obsidiana |
| `specular_strength` | 0.9 | 0 – 3 | Intensidad del brillo |
| `shininess` | 40 | 2 – 256 | Exponente de Blinn-Phong (mayor = brillo más concentrado) |
| `night_lights` | 1 | 0 – 1 | Fisuras más brillantes de noche |
| `night_glow` | 1 | 0 – 3 | Emisión nocturna de las fisuras |
| `terminator_width` | 0.25 | 0.02 – 0.8 | Suavidad del paso del día a la noche |
| `parallax` | 1 | 0 – 1 | Parallax dependiente de la vista (tecla `P`) |
| `parallax_scale` | 0.06 | 0 – 0.25 | Profundidad aparente del relieve |
| `parallax_steps` | 8 | 2 – 32 | Pasos de la búsqueda (redondeado) |

### House of Cards

| Parámetro | Por defecto | Rango | Cambio visual |
|---|---:|---|---|
| `seed` | 23 | 0 – 9999 | Otras ondulaciones y tormentas |
| `band_count` | 6 | 3 – 14 | Número de bandas |
| `band_warp` | 0.25 | 0 – 1.5 | Ondulación de los bordes de banda |
| `band_flow` | 0.12 | 0 – 1 | Viento medio (rad/s) |
| `differential_wind` | 0.06 | 0 – 1.5 | Diferencia de viento entre latitudes |
| `filament_strength` | 0.15 | 0 – 0.5 | Filamentos finos en las bandas |
| `rim_strength` | 0.6 | 0 – 2 | Halo violeta y magenta del limbo |
| `wrap` | 0.35 | 0 – 1 | Iluminación envolvente de las nubes |
| `vortex` | 1 | 0 – 1 | Tormentas ♦ |
| `vortex_period` | 18 | 4 – 90 | Segundos por ciclo de la tormenta |
| `vortex_radius` | 0.45 | 0.1 – 0.5 | Semidiagonal del rombo |
| `vortex_strength` | 6 | 0 – 8 | Circulación del núcleo, rad (tecla `[` / `]`) |
| `vortex_aspect` | 1.35 | 0.6 – 2 | Diagonal norte-sur sobre la este-oeste |
| `secondary_strength` | 0.6 | 0 – 2 | Circulación de la tormenta secundaria |
| `vortex_max_latitude` | 45 | 0 – 60 | Latitud máxima de los centros (grados) |
| `lightning` | 1 | 0 – 1 | Relámpagos neón en las tormentas |
| `lightning_rate` | 1 | 0 – 3 | Frecuencia de los relámpagos |
| `lightning_intensity` | 1.4 | 0 – 3 | Brillo de los relámpagos |

## 8. Pruebas por criterio

| Criterio | Archivos de prueba |
|---|---|
| B01 / E01 | `lab6_crown_tests.rs`: emisiva, paleta cálida, corrientes y venas, cambio con cámara y malla fijas, histograma no rígido, manchas que respiran, pulso, eclipse, ninguna figura de diadema |
| B02 | `lab6_entrance_terrain_tests.rs` y `lab6_entrance_events_tests.rs`: siete regiones con la roca oscura dominante y la lava solo en los cráteres, cuenca, cráter-pica: un solo sistema de tres ranuras, cuenca con forma de ♠, brillo tras la profundidad máxima, continuidad, separación y zonas recientes |
| B03 / E05 | `lab6_house_tests.rs`: varianza por latitud, ondulación solo con *warp*, costuras, vórtice estrictamente local, ida y vuelta del contorno ♦, migración |
| E02 / E03 | `lab6_entrance_lighting_tests.rs`: emisión nocturna ≥ 3×, monotonía, obsidiana dependiente de la vista, roca independiente, fisuras independientes de L |
| E04 / E08 | `lab6_entrance_clouds_tests.rs`: cobertura monótona, rotación rígida exacta, sin geometría nueva, sombra = máscara en q, nula en el punto subsolar, sin sombra de noche |
| E06 / E07 | `lab6_entrance_relief_tests.rs`: identidad sin bump, inclinación, luces opuestas, parallax hacia la vista en colinas y en sentido contrario en cráteres, desvanecimiento en el limbo, tiempo congelado |
| Restricciones y regresión | `lab6_regression_guard_tests.rs` (21 *hashes* de la nave), `lab6_sphere_asset_tests.rs`, `lab6_final_contract_tests.rs` (sin APIs de dibujo ni texturas, símbolos exclusivos, dependencias, *assets*), `lab6_shaded_pipeline_tests.rs` |
| B05 | `lab6_cli_tests.rs`, `lab6_timeline_record_tests.rs`, `lab6_docs_evidence_tests.rs` |

## 9. Reproducción de evidencias

- **Capturas 01-04:** salen de la ventana real (`--screenshot`).
- **Montajes 05-09:** se componen con fotogramas sin ventana:
  ```bash
  ./target/release/rust-software-renderer <args> --record 0.04 --fps 25 | \
    ffmpeg -f rawvideo -pix_fmt rgb24 -s 800x600 -framerate 25 -i - -frames:v 1 panel.png
  ```
  Los paneles se unen con `hstack`.
- **Videos:** se regeneran con [`docs/videos/lab6/encode.sh`](videos/lab6/encode.sh) `all`, que contiene la tabla exacta de argumentos. El flujo:
  1. `--record` produce rgb24.
  2. ffmpeg lo convierte en un intermedio FFV1 sin pérdida a 30 fps (declarado con `-framerate`).
  3. Del intermedio salen el MP4 H.264 y el GIF.
- **Codificador:** el script lo elige con una prueba funcional. En esta máquina fue `h264_nvenc` (`-rc constqp -qp 32 -bf 0`, sin AQ, para que cada archivo quede por debajo de unos 4 MB; `QP=34` en la órbita del clip 06, que cambia todos los píxeles); `libx264` no está disponible. Los GIF van a 8 fps, con 64 colores y 320 px de ancho (640 en las comparaciones lado a lado). Sin codificador H.264, `ALLOW_GIF_ONLY=1` produce solo GIF.
- **Aislamiento:** los clips de bump (05), parallax (06) y respuestas a la luz (08) se graban con `--set clouds=0` para aislar el efecto. El de parallax usa además la escala de demostración `parallax_scale=0.12`.
- **Líneas de tiempo:**

  | Línea de tiempo | Qué hace |
  |---|---|
  | `material` | Cámara y luz fijas |
  | `orbit` | Una vuelta de cámara con el tiempo congelado |
  | `light` | Una vuelta del azimut de la luz con la cámara fija; con `--freeze-time` también el tiempo |
  | `showcase` | Un cuarto de vuelta |

## 10. Rendimiento

Medido con `cargo run --release -- --benchmark-shaders` a 800×600 y t = 6 s. Son medianas de 5 rondas en una CPU con 20 hilos; los datos están en [`docs/results/lab6/shader_benchmark.csv`](results/lab6/shader_benchmark.csv).

| Cuerpo | Modo | 1 hilo (ms) | 20 hilos (ms) |
|---|---|---:|---:|
| The King's Crown | scanline | 251.7 | 63.1 |
| Crimson Entrance | scanline | 1414.8 | 269.0 |
| House of Cards | scanline | 93.6 | 30.5 |

Los tres cuerpos sombrean 172 152 fragmentos por cuadro, uno por píxel visible gracias al culling. Crimson Entrance es el más costoso porque cada fragmento hace:
- la búsqueda de parallax (11 alturas macro);
- el gradiente del bump (4 alturas completas);
- las regiones;
- la máscara de nubes y la de la sombra.

En la ventana iba a unos 4 cuadros por segundo; los videos se graban sin ventana y no dependen de esa velocidad. El benchmark del Lab 5 y sus CSV no cambiaron.

Esta tabla es la medición original. Tras el reajuste de Crimson Entrance (bump con 2 alturas, parallax adaptativo, ruido sin saltos mal predichos y cola de bloques para los hilos), en scanline baja a 713.9 ms con 1 hilo y a 61.6 ms con 20, unos 15 cuadros por segundo. Ver [lab6-crimson-refinement.md](lab6-crimson-refinement.md) y [`crimson_refinement_benchmark.csv`](results/lab6/crimson_refinement_benchmark.csv).

## 11. Limitaciones

- **Silueta poligonal.** La esfera tiene 98 vértices: su sagita máxima es el 4.3 % del radio. Por regla no se modifican vértices; el detalle sale del fragmento.
- **Relieve aparente.** Cráteres-pica y cuencas son bump, parallax y sombreado: la silueta no muestra hundimientos.
- **Parallax aproximado.** Es una búsqueda escalonada y se atenúa a propósito cerca del limbo.
- **Diferido al proyecto final.** El anillo de cartas ♦, el halo exterior de la estrella y la cámara perspectiva necesitan geometría o pases propios.
