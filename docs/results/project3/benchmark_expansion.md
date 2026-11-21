# Proyecto 3 — Benchmark de la expansión (2026-11-20)

Este documento mide cuánto cuesta por cuadro la expansión del 2026-11-20:

- el Joker;
- el cielo que cicla de morado a azul;
- los cometas;
- la captura orbital;
- las cascadas monumentales y el oleaje;
- las luces automáticas;
- el audio procedural.

Hay dos mediciones:

1. **Antes y después:** el código de `3bc7ad3` (cierre de C80) frente al de `4f3b3af` (último commit de código de la expansión) en los mismos escenarios.
2. **Costo de cada función:** cada función encendida y apagada en la misma escena y en el mismo proceso.

Todos los valores son medidos. Lo que no se midió se dice al final.

## Condiciones

- **Máquina:** Intel Core i7-13650HX, 20 hilos lógicos, Fedora 43 (kernel 7.1.5). Es la misma laptop que [benchmark_final.md](benchmark_final.md).
- **Escenas:** `scenario_scene_on(id, …, true)` de `src/benchmark/solar.rs`, es decir, el sistema del Proyecto 3 a su escala. Se usa la configuración por defecto: 800×600, t = 6 s, semilla 42 y relleno scanline.
- **Qué mide:** el cuadro de CPU (`SolarScene::render_frame`), sin la subida a la ventana.
- **Programa de medición:** es un programa temporal fuera del repositorio que solo llama a esas funciones públicas, así que no forma parte de la entrega.
- **Antes y después:**
  - `3bc7ad3` se extrajo con `git archive` y se compiló en `--release` aparte;
  - cada build corrió los mismos escenarios en el orden base, cabeza, base, cabeza, primero con 1 hilo y luego con 20;
  - en cada corrida hubo 2 cuadros de calentamiento y 9 medidos;
  - la tabla da la media de las dos medianas y, entre paréntesis, el rango.
- **Costo de cada función:**
  - las dos variantes de la escena (con la función y sin ella) se alternan cuadro a cuadro en el mismo proceso, durante 21 rondas;
  - se da la mediana de cada variante;
  - «Píxeles» cuenta los píxeles que cambian entre las dos variantes y confirma que la función se ve en ese encuadre.

## Antes y después de la expansión

En ms por cuadro. «Cociente» es después / antes.

| Escenario | Antes, 1 hilo | Después, 1 hilo | Cociente | Antes, 20 hilos | Después, 20 hilos | Cociente |
|---|---:|---:|---:|---:|---:|---:|
| D sistema completo con la nave | 22.22 (22.18–22.26) | 22.77 (22.75–22.79) | 1.02 | 15.74 (15.29–16.19) | 16.73 (16.26–17.19) | 1.06 |
| C vista orbital | 8.89 (8.87–8.92) | 7.59 (7.59–7.60) | 0.85 | 7.18 (7.16–7.19) | 7.88 (7.52–8.24) | 1.10 |
| E cerca de Crimson Entrance | 316.13 (315.88–316.39) | 316.38 (316.00–316.77) | 1.00 | 48.81 (39.52–58.10) | 43.22 (38.24–48.19) | 0.89 |
| F cerca de House of Cards y su anillo | 51.81 (51.74–51.88) | 52.62 (52.62–52.62) | 1.02 | 19.88 (14.69–25.07) | 23.43 (22.80–24.05) | 1.18 |
| J vuelo más bajo sobre Crimson Entrance | 662.62 (662.11–663.13) | 650.74 (650.34–651.14) | 0.98 | 75.60 (72.52–78.69) | 74.56 (74.02–75.10) | 0.99 |
| K tierra del trébol | 182.64 (182.59–182.70) | 188.94 (185.58–192.30) | 1.03 | 39.37 (38.55–40.19) | 38.16 (36.86–39.47) | 0.97 |
| K3 montaña y cascada del trébol | 143.75 (143.45–144.05) | 149.24 (148.00–150.48) | 1.04 | 37.55 (36.75–38.35) | 40.42 (37.77–43.06) | 1.08 |
| M pirámide y ciudad de Neon Hearts, con tráfico | 228.56 (228.40–228.71) | 230.32 (229.98–230.67) | 1.01 | 48.46 (48.03–48.89) | 47.84 (47.79–47.90) | 0.99 |
| M2 Neon Hearts a 12 radios | 32.95 (32.90–33.00) | 35.26 (33.55–36.97) | 1.07 | 17.67 (16.02–19.33) | 18.72 (18.52–18.92) | 1.06 |
| L3 relieve rasante, LOD automático | 732.41 (725.02–739.81) | 727.30 (727.09–727.50) | 0.99 | 84.64 (83.68–85.59) | 86.09 (82.52–89.65) | 1.02 |
| N Joker de cerca | — | 93.42 (93.33–93.51) | — | — | 38.57 (38.07–39.07) | — |
| N2 Joker a 10 radios | — | 10.59 (10.58–10.59) | — | — | 7.14 (7.07–7.21) | — |

Contadores del último cuadro después de la expansión, con su valor anterior entre paréntesis:

| Escenario | Cuerpos | Objetos probados / descartados | Dibujos | Triángulos | Fragmentos | LOD | Memoria pico (KB) |
|---|---:|---|---:|---:|---:|---|---:|
| D | 6 (5) | 8 / 0 | 47 | 5072 (4752) | 58 000 (57 579) | clover L2, hearts L2, joker L2 | 29 936 (26 948) |
| E | 6 (5) | 8 / 3 | 24 | 3680 (3680) | 119 049 (119 049) | hearts L2 | 36 948 (34 084) |
| F | 6 (5) | 8 / 5 | 14 | 1630 (1630) | 128 345 (128 345) | clover L2 | 38 228 (34 560) |
| J | 6 (5) | 8 / 2 | 25 | 4000 (3680) | 268 574 (267 834) | hearts L2, joker L2 | 50 180 (45 872) |
| K3 | 6 (5) | 8 / 6 | 11 | 26 630 (26 630) | 270 877 (270 877) | clover L5 | 53 332 (50 128) |
| M | 6 (5) | 8 / 6 | 43 | 39 408 (39 408) | 274 930 (274 930) | hearts L4 | 52 156 (49 640) |
| N | 6 | 8 / 5 | 26 | 6766 | 201 138 | joker L4 | 52 156 |

Lectura:

- **Sin regresión fuera del ruido:**
  - con 1 hilo, los cocientes van de 0.98 a 1.07;
  - con 20 hilos, van de 0.89 a 1.18, con rangos de varios ms en ambos lados.
- **Encuadres que se movieron:**
  - con el Joker en la órbita 49, el radio del mundo crece y la vista orbital (C) se aleja;
  - por eso C dibuja menos fragmentos (4099 frente a 5512) y baja con 1 hilo.
- **El Joker:**
  - en D añade 320 triángulos (nivel L2) y unos 400 fragmentos;
  - de cerca (N) cuesta 93 ms con 1 hilo, la mitad que la tierra del trébol (K, 189 ms);
  - con 20 hilos cuesta 39 ms, como K (38 ms).
- **Memoria:**
  - la memoria pico del proceso (`VmHWM`, acumulada dentro de cada corrida) crece entre 2.5 y 4.3 MB por escenario;
  - lo más probable es que venga de la geometría del Joker y su relieve, pero no se separó por función.
- **El peor caso** sigue siendo el relieve rasante de C75: L3 mide 86.1 ms con 20 hilos (82.8 ms en C75).

## Costo de cada función

| Función | Escenario | Con ella, 1 hilo | Sin ella, 1 hilo | Con ella, 20 hilos | Sin ella, 20 hilos | Píxeles |
|---|---|---:|---:|---:|---:|---:|
| Cometas | D | 22.93 | 22.87 | 16.57 | 16.62 | 663 |
| Cometas | C | 7.59 | 7.54 | 7.69 | 7.70 | 971 |
| Cascadas monumentales | K3 apuntando a una monumental | 190.83 | 190.70 | 55.32 | 46.06 | 663 |
| Cascadas monumentales | K3, K, K2 | iguales ± 0.3 | | ± 10 de ruido | | 0 (no se ven) |
| Luces automáticas (sombra 1.0) | E | 316.53 | 316.44 | 42.07 | 39.16 | 3010 |
| Luces automáticas (sombra 1.0) | F | 52.56 | 52.45 | 24.29 | 23.57 | 2975 |
| Luces automáticas (sombra 1.0) | L3 | 729.47 | 729.17 | 82.85 | 83.31 | 2959 |

- **Cometas:** cuestan menos de 0.15 ms por cuadro.
- **Cascadas monumentales:**
  - cuestan unos 0.1–0.3 ms con 1 hilo cuando se ven, y 0 píxeles cuando no;
  - con 20 hilos las diferencias van de −10 a +9 ms en los dos sentidos, incluso en encuadres donde la cascada no cambia ningún píxel, así que son ruido del reparto entre hilos;
  - el oleaje no se puede apagar por separado: va dentro del costo de K y K3 en la tabla de antes y después (+3–4 % con 1 hilo).
- **Luces automáticas:**
  - en los cuatro vuelos estacionarios medidos (E, F, J y L3) la nave está en sombra total, así que las luces están encendidas;
  - cuestan menos de 0.3 ms con 1 hilo (en J la variante sin luces salió 5.4 ms más lenta, que es ruido);
  - con 20 hilos van de −0.7 a +2.9 ms, también dentro del ruido.
- **Captura orbital:**
  - el controlador cuesta 0.66 µs por paso en órbita asistida, frente a 0.57 µs con la captura apagada;
  - son 20 000 pasos a 60 FPS, con la nave capturada por Clover Haven;
  - no toca el cuadro de render.
- **Audio procedural:**
  - los tres sonidos se sintetizan una sola vez, al abrir la ventana, en unos 25 ms;
  - ocupan 128 022 bytes de WAV en memoria;
  - por cuadro solo se ajustan dos volúmenes.
- **Cielo morado → azul:** no se puede apagar por separado (solo cambia colores con el tiempo, con el mismo número de operaciones). Va dentro de D en la tabla de antes y después.

## Qué no se midió

- **Escape tangencial:** su costo por paso no se midió aparte. Usa el mismo paso espacial que el vuelo libre.
- **Escena completa en la ventana en vivo:** la subida de la textura, la mezcla de audio de raylib y los FPS reales no se midieron; esta medición es solo del cuadro de CPU.
- **Escenarios forzados:** no se repitieron los escenarios con LOD forzado (`L1F`–`L3F`) ni los de sin descarte. Siguen valiendo los de [benchmark_final.md](benchmark_final.md).
- **Reescalado:** el reescalado del universo no se aplicó, así que no se midió. Ver [la evaluación](../../project3_rescale_evaluation.md).
