# Estudio C37 — Planificación del sombreado por cuadro

**Resultado: adoptado.** El cuadro de la escena solar se sombrea con un solo grupo de hilos en lugar de uno por instancia. La imagen es idéntica bit a bit y el cuadro baja hasta un 31 % con todos los hilos.

## Hipótesis

Con un `thread::scope` por instancia, al final de cada cuerpo los 20 hilos esperan a que terminen los últimos bloques de 512 fragmentos (unos 2 ms de trabajo en Crimson Entrance a un hilo). Con 3 cuerpos más la nave por cuadro, esas colas ociosas se repiten cuatro veces. Una sola cola por cuadro las solapa.

## Diseño adoptado

`SolarScene::render_frame` en modos rellenos:

1. limpia, dibuja las estrellas;
2. rasteriza cada instancia visible con `RenderScratch::rasterize`, escribiendo la profundidad de cada muestra aceptada en el acto (rechazo temprano para las instancias siguientes) y etiquetándola con su trabajo;
3. sombrea todas las muestras con `RenderScratch::shade_batch` (una sola cola);
4. escribe cada color solo donde la muestra ganó la prueba de profundidad (`Framebuffer::point_if_depth`).

Los dos rasterizadores (`raster_orbit`, `raster_perspective`) son los de antes extraídos a un sumidero común: no hay bucles duplicados y `render_shaded` / `render_shaded_with` del camino por instancia siguen intactos (oráculos C01 y del Laboratorio 6 sin cambios). El wireframe sigue dibujando instancia por instancia.

## Corrección

- `the_frame_batch_matches_drawing_the_instances_one_by_one`: 6 escenarios × 2 proyecciones × 2 rellenos × 1 y 20 hilos = 48 imágenes idénticas al dibujo instancia por instancia.
- El lote nunca sombrea más fragmentos que el dibujo separado (la profundidad escrita antes evita sombrear lo que una instancia anterior ya tapa).

## Medición

Prototipo fuera del repositorio primero, y luego el código de producción. A/B en un mismo proceso con el orden alternado en cada iteración (160 cuadros por variante, 800×600, 20 hilos, mediana):

| Escenario | Instancia por instancia (ms) | Lote por cuadro (ms) | Cambio |
|---|---:|---:|---:|
| A nave sola | 6.70 | 6.37 | −4.9 % |
| B un planeta | 21.93 | 21.44 | −2.2 % |
| C sistema lejano | 14.11 | 9.66 | −31.5 % |
| D sistema con nave | 20.26 | 14.93 | −26.3 % |
| E acercamiento a Entrance | 34.45 | 29.94 | −13.1 % |
| F acercamiento a House | 24.96 | 19.63 | −21.4 % |
| G mirando al vacío | 6.53 | 6.16 | −5.8 % |
| J vuelo mínimo sobre Entrance | 44.86 | 43.39 | −3.3 % |

Con un solo hilo el prototipo no cambiaba (entre −4 % y +1 %), lo que confirma que la ganancia viene del reparto entre núcleos y no de menos trabajo.

### Error de medición corregido

La primera A/B midió siempre la variante por instancia antes que la de lote y mostró J +8 % a +12 %. Con el orden alternado esa regresión desaparece (−2.3 % en el prototipo, −3.3 % en producción): era un sesgo del orden de medición, no del planificador.

## Criterio del Gate

Mejora ≥ 5 % en el escenario D (−26.3 %), sin regresión en ningún escenario e imágenes idénticas: se integra en producción. `solar_benchmark.csv` se regeneró con el código adoptado; las medianas anteriores quedan en el README de esta carpeta.
