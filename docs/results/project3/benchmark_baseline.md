# Proyecto 3 — Línea base de rendimiento

Costo de CPU por cuadro de la escena solar **antes** de añadir geometría nueva. Es la referencia contra la que se comparan los cambios del Proyecto 3.

## Condiciones

- **Commit medido:** `0f26c06`, el código de producción de `ea3b4a8` más este commit, que solo añade columnas y `--threads` al benchmark.
- **Máquina:** Intel Core i7-13650HX, 20 hilos lógicos; rustc 1.97.0; Fedora 43 (kernel 7.1.5).
- **Comandos:**
  - `cargo run --release -- --benchmark-solar --threads 1 --benchmark-output <dir>`;
  - el mismo comando sin `--threads`, para todos los núcleos.
- **Escena:** 800×600, t = 6 s, semilla 42, relleno scanline, 1 calentamiento + 7 rondas por corrida.
- **Repeticiones:** 3 corridas por configuración, en orden alterno (1 → 20, 20 → 1, 1 → 20) para no favorecer a ninguna.
- **Qué mide:** el cuadro de CPU (`SolarScene::render_frame`). No incluye `to_rgba_bytes` ni la subida de la textura a la ventana.
- **Datos completos:** `solar_benchmark_baseline.csv`. Por escenario e hilos: mediana de las medianas, rango de las medianas, ronda mínima y máxima, y memoria.

## Resultados (ms por cuadro, mediana de 3 medianas; entre paréntesis, el rango de las medianas)

| Escenario | 1 hilo | 20 hilos | Fragmentos sombreados |
|---|---:|---:|---:|
| A nave sola | 5.22 (5.11–6.15) | 9.12 (8.74–9.28) | 25 993 |
| B un planeta | 85.62 (85.26–85.70) | 22.68 (21.45–23.92) | 23 469 |
| C sistema lejano | 17.07 (17.03–18.31) | 10.94 (10.94–11.39) | 13 183 |
| D sistema con nave | 31.73 (31.64–31.86) | 17.14 (12.28–17.32) | 43 312 |
| E acercamiento a Crimson | 130.00 (129.99–136.41) | 21.25 (20.95–32.74) | 62 738 |
| F acercamiento a House | 56.89 (56.86–57.23) | 22.10 (16.09–23.75) | 108 900 |
| G mirando al vacío | 5.14 (5.12–5.19) | 8.21 (8.09–8.94) | 25 993 |
| H sistema con estrellas | 31.72 (31.63–42.49) | 17.48 (12.47–17.87) | 43 312 |
| I sistema sin cielo | 30.08 (30.02–30.17) | 15.86 (10.51–16.39) | 43 312 |
| J vuelo más bajo sobre Crimson | 286.58 (286.45–293.58) | 40.25 (39.95–41.08) | 132 873 |

## Lectura

- **El peor caso sigue siendo J:** 286 ms con un hilo y 40 ms con 20 hilos, unos 25 FPS de CPU.
- **A y G cuestan más con 20 hilos que con 1:** la escena es tan ligera que arrancar el pool de sombreado pesa más que el trabajo.
- **Memoria:** la columna `peak_rss_kb` es el pico del proceso (`VmHWM`) acumulado hasta ese escenario, no la memoria propia de cada uno. Crece de unos 14 MB a unos 30 MB a lo largo de la corrida.
- **Ruido:** la variación entre corridas llega a ±25 % en esta laptop por el escalado de frecuencia de la CPU. Aquí se ve, por ejemplo, en el rango de D con 20 hilos. Ninguna mejora futura se afirma sin un A/B intercalado.
- **Sin cifras de FPS de ventana:** solo ms de CPU por cuadro.
