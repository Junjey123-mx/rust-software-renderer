# Proyecto 3 — Capturas y animaciones de la expansión

Todas las imágenes de la expansión del 2026-11-20 son salidas reales del rasterizador por CPU de este repositorio: no hay retoques, composiciones pintadas ni imágenes conceptuales. Las generó un arnés de pruebas fuera del repositorio que solo llama a la API pública del *crate*:

- `benchmark::solar::scenario_scene_on` arma los escenarios medibles, a `t = 6 s` salvo que se indique otro tiempo;
- `solar::viewer::step` hace volar la nave;
- `SolarScene::render_frame` dibuja.

Cada cuadro se guardó como PPM y se convirtió con Pillow (PNG) o con ffmpeg: paleta de 80–96 colores con *dither* Bayer (GIF). Las capturas 01–08 son anteriores y su origen está en [project3_final_validation.md](../../project3_final_validation.md).

## Capturas (`docs/screenshots/project3/`)

| Archivo | Cómo se obtuvo |
|---|---|
| `09-orbital-capture.png` | 800×600, cámara Follow. Escenario `D` a `t = 16 s`, con la captura activada y la nave en el lado de Clover Haven que mira a la estrella. Va a 0.8 u/s hacia el planeta, con los amortiguadores apagados, 0.15 R fuera de la cáscara de captura. Es el cuadro 30 (2 s a 15 cuadros/s), ya en `ORBITAL CAPTURE`. La etiqueta es el HUD del visor (`draw_hud`) |
| `10-tangential-escape.png` | La misma secuencia, cuadro 124: `SPACE` se mantiene desde el cuadro 120 (`TANGENTIAL ESCAPE`, postcombustión azul) |
| `11-clover-monumental-fall.png` | 800×600, cámara Orbit sobre la cascada monumental 1 de Clover Haven, a 0.55 R. Escenario `K3` a `t = 16 s`, el tiempo de 0–70 s en que esa cascada mira más hacia la estrella |
| `12-ship-lights.png` | Escenario `E` (800×600): la nave está a la sombra de Crimson Entrance. Se ven las luces roja y verde de las alas, el estrobo de la cola y el haz delantero |
| `13-sky-purple-blue.png` | Escenario `G` (la nave frente al espacio vacío) a `t = 0 s` y `t = 45 s`, lado a lado (1600×600) |
| `14-comet.png` | 800×600: una cámara a 30 u del cometa 0 en `t = 10 s` |
| `15-live-window.png` | Captura de la ventana del visor interactivo (`--scene solar --screenshot`) en su primer cuadro, con el audio activo en esa ejecución |

## Animaciones (`docs/videos/project3/`)

| Archivo | Cómo se obtuvo |
|---|---|
| `02-orbital-capture.gif` | La secuencia de las capturas 09–10 a 400×300, 150 pasos de 1/15 s: aproximación, captura, órbita asistida y escape con `SPACE` desde el paso 120. Se reduce a 360 px y 7.5 cuadros/s |
| `03-monumental-waterfall.gif` | Cámara Orbit sobre la cascada monumental 1, a 0.38 R. 40 cuadros entre `t = 16` y 21 s, a 400×300 |
| `04-clover-ocean.gif` | Cámara Orbit sobre Clover Haven (1.3 R de encuadre). 40 cuadros entre `t = 40` y 56 s: olas, oleaje, espuma y cascadas |
| `05-joker.gif` | 25 cuadros a 400×300. Cámara Orbit sobre el Joker: giro del planeta y animación del shader de arlequín |
| `06-sky-cycle.gif` | 31 cuadros a 400×300. Un escenario con la nave y la estrella reconstruido a lo largo del ciclo del cielo: la nebulosa pasa de púrpura a azul y vuelve |
| `07-comet.gif` | 24 cuadros a 400×300. Cámara Follow detrás de la nave mientras un cometa cruza el cielo |
| `08-house-ring.gif` | 30 cuadros a 400×300. Cámara Orbit sobre House of Cards: el anillo de cartas gira y se mece |
| `09-hearts.gif` | 32 cuadros a 400×300. La nave sobre Neon Hearts: rejilla, *glitch*, pirámide-corazón, ciudad, naves y motos de luz |
| `10-star.gif` | 30 cuadros a 400×300. Cámara Orbit sobre The King's Crown: plasma, corona y lazos de plasma |
| `11-thrusters.gif` | 54 cuadros a 400×300. Cámara Follow: crucero, velocidad de la luz y apagado de los propulsores |

Las animaciones 05–11 se generaron durante el trabajo de los cometas, antes de las cascadas monumentales, el oleaje y las luces automáticas, y no muestran Clover Haven de cerca. La nave puede verse en ellas sin las luces, que se encienden solo a la sombra de un planeta.

Los recorridos `00-provisional-tour.mp4` y `01-final-tour.mp4`/`.gif` son evidencia histórica de la versión de cuatro planetas. **No son el video final de esta expansión**, que grabará el autor (sección «Video de demostración» del README).

## Segunda pasada: capturas 16–22 (escala Grand)

Se generaron el 2026-11-20 con el código final de la segunda pasada (commit `03596af`) y una herramienta temporal fuera del repositorio. La herramienta construye `SolarScene::grand_system` a 800×600, pone la cámara Follow, coloca la nave respecto de un planeta y dibuja con `render_frame` en el rasterizador por CPU, con todos los núcleos.

| Archivo | Encuadre | Mediana de 5 cuadros |
|---|---|---:|
| `16-grand-spawn.png` | Punto de aparición a `t = 6` s, mirando hacia la estrella | 12.9 ms |
| `17-grand-clover-sea.png` | A 1.9 R de Clover Haven, del lado iluminado | 52.8 ms |
| `18-clover-monumental-curtain.png` | A 0.4 R de la segunda cascada monumental, de costado, en el instante en que mira a la estrella (`t = 65.25` s) | 86.6 ms |
| `19-ace-crystal.png` | A 1.9 R de Ace Crystal, del lado iluminado | 53.6 ms |
| `20-joker-harlequin.png` | A 2 R del Joker, del lado iluminado | 60.2 ms |
| `21-headlight-night.png` | Vuelo rasante a 1.12 R en la cara de noche de Crimson Entrance, con las luces encendidas (sombra 1.0) | 114.3 ms |
| `22-ship-bank.png` | A 2.4 R de House of Cards con un alabeo fijado en 0.5 rad | 24.9 ms |

En la captura 22 el alabeo se fijó a mano para la foto. En el visor lo produce el controlador al pulsar `A`/`D` o girar con el ratón.

## Tercera pasada: capturas 23–26 y animaciones 12–13 (vuelo atmosférico)

Se generaron el 2026-11-21 con el código del commit `64a6fec`, copiado con `git archive` fuera del repositorio. El commit siguiente, `784a29e`, solo cambia el texto del HUD y de la ayuda, que `render_frame` no dibuja.

**Herramientas temporales** (fuera del repositorio):

- `gshot`, la de la segunda pasada, para las vistas fijas desde lejos;
- `fly`, que conduce el controlador real del visor (`viewer::step` a 60 FPS, con `capture.enabled`, como `--scene solar`) y dibuja con `render_frame` en el rasterizador por CPU con todos los núcleos:
  - **escenario de picada:** la nave empieza 0.5 u sobre la atmósfera, a 8 u/s hacia el planeta, con la nariz en la tangente;
  - después avanza con `W` y sale con `SPACE` + `W`.

| Archivo | Qué muestra |
|---|---|
| `23-atmosphere-halos.png` | Los seis planetas a 3.2 R, del lado iluminado (`gshot`, 800×600 cada uno, reducido al 40 %): el halo de cada aire, más brillante de día |
| `24-atmospheric-flight.png` | Los seis planetas 2 s después de empezar a avanzar en vuelo atmosférico (`fly`, escenario de picada, 800×600 cada uno, reducido al 40 %) |
| `25-atmospheric-entry.png` | Clover Haven en el mismo vuelo a los 1.0, 3.5 y 7.5 s: la entrada, el vuelo rasante de día y la cara de noche |
| `26-pov-recovery-before-after.png` | El mismo escape desde la captura ecuatorial de House of Cards, 2.5 s después de pulsar el turbo, con el código de `bec7319` (izquierda) y el de `64a6fec` (derecha). A la izquierda el Joker del fondo aparece de lado porque la vista sigue girada 90° con el planeta. A la derecha está derecho y tiene además el halo nuevo |
| `12-atmospheric-flight-clover.gif` | Clover Haven a 480×360 y 12 cuadros/s: picada, vuelo rasante (3 s), turbo y recuperación de la vista hasta el Follow estable |
| `13-escape-pov-recovery-hearts.gif` | Neon Hearts en el hemisferio sur, igual que la anterior. Al pulsar el turbo, el «arriba» de la nave está girado unos 115° respecto del espacio (`up·space_up` = −0.41) y llega a 1.000 a los 0.83 s; la recuperación termina a los 0.9 s |

**Registros de las herramientas** (no se versionan):

- **Altura en el vuelo rasante:** de 1.20 a 1.22 u sobre el mar de Clover Haven, y de 1.12 a 1.21 u sobre la ciudad de Neon Hearts. La altura objetivo es 1.20 u.
- **El escape de la captura 26:** la distancia al planeta y la velocidad son iguales cuadro a cuadro en las dos versiones.
