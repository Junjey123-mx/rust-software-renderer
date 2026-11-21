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
