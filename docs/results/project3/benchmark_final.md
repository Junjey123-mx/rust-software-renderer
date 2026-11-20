# Proyecto 3 — Benchmark final (C75)

Este es el costo de CPU por cuadro de la escena completa del Proyecto 3: cuatro planetas, adornos, modelos de superficie, cascadas y tráfico. Se mide con descarte y sin él, con LOD automático y con el nivel 5 forzado, en 1 hilo y en 20. Se compara con la línea base de C02 (`benchmark_baseline.md`). El quinto planeta no existe (`ART_PENDING_APPROVAL`), así que no hay escenario para él.

## Condiciones

- **Commit medido:** `a3d4c3f` (el código de producción de C74) más este commit, que solo añade el arnés del benchmark final.
- **Máquina:** Intel Core i7-13650HX, 20 hilos lógicos; Fedora 43 (kernel 7.1.5). Es la misma laptop que la línea base.
- **Escena:** 800×600, t = 6 s, semilla 42, relleno scanline, 1 calentamiento + 7 rondas.
- **Repeticiones:** 3 corridas por número de hilos. Cada corrida recorre los 52 casos, y la segunda los recorre al revés para que ningún caso vaya siempre primero o último. Primero se mide 1 hilo y después todos los núcleos (20).
- **Casos:**
  - A–J sobre el sistema del preproyecto (`legacy`);
  - A–J, L1–L3F, K–K3 y M–M2 sobre el sistema del Proyecto 3 a su escala (`p3`);
  - todos los `p3` otra vez sin descarte (`no-culling`).
- **Qué mide:** el cuadro de CPU (`SolarScene::render_frame`). No incluye la subida a la ventana.
- **Datos completos:** `solar_benchmark_final.csv`. Por caso e hilos:
  - la mediana de las 3 medianas y el rango de las medianas;
  - la ronda mínima y la máxima;
  - la memoria;
  - los contadores del último cuadro (objetos probados y descartados, dibujos, triángulos enviados y descartados por el rasterizador);
  - el nivel de detalle de cada cuerpo.
- **Para regenerarlo** (unos 5 minutos):

```bash
P3_COMMIT=<commit> P3_FINAL_CSV=docs/results/project3/solar_benchmark_final.csv \
  cargo test --release --test p3_benchmark_csv_tests regenerate_final_csv -- --ignored
```

`tests/p3_benchmark_csv_tests.rs` (`final_csv_has_every_case`) comprueba que el CSV tiene los 52 casos con 1 hilo y con todos los núcleos, 3 corridas de 7 rondas y un rango ordenado. Si falta un escenario, la prueba falla.

## Sistema del preproyecto frente a la línea base de C02

Son ms por cuadro: la mediana de las 3 medianas y, entre paréntesis, el rango de las medianas. El cociente es final / C02.

| Escenario | C02, 1 hilo | Final, 1 hilo | Cociente | C02, 20 hilos | Final, 20 hilos | Cociente |
|---|---:|---:|---:|---:|---:|---:|
| A | 5.22 | 5.19 (5.17–8.51) | 0.99 | 9.12 | 7.16 (6.29–10.54) | 0.79 |
| B | 85.62 | 86.12 (86.01–86.31) | 1.01 | 22.68 | 19.88 (13.90–22.24) | 0.88 |
| C | 17.07 | 17.42 (17.38–17.51) | 1.02 | 10.94 | 10.98 (7.87–13.79) | 1.00 |
| D | 31.73 | 32.70 (32.68–33.13) | 1.03 | 17.14 | 18.21 (12.59–19.28) | 1.06 |
| E | 130.00 | 131.24 (131.16–131.41) | 1.01 | 21.25 | 22.02 (21.27–27.69) | 1.04 |
| F | 56.89 | 57.16 (56.88–76.72) | 1.00 | 22.10 | 15.99 (15.76–20.59) | 0.72 |
| G | 5.14 | 5.15 (5.15–5.19) | 1.00 | 8.21 | 8.86 (6.20–10.26) | 1.08 |
| H | 31.72 | 32.72 (32.64–32.73) | 1.03 | 17.48 | 18.24 (12.21–19.22) | 1.04 |
| I | 30.08 | 31.04 (31.04–31.11) | 1.03 | 15.86 | 16.47 (16.13–16.62) | 1.04 |
| J | 286.58 | 291.72 (291.51–292.02) | 1.02 | 40.25 | 42.15 (40.70–58.65) | 1.05 |

- **Con 1 hilo:** todo queda entre 0.99 y 1.03 de la línea base.
- **Con 20 hilos:** entre 0.72 y 1.08. Es el ruido de ±25 % de esta laptop (escalado de frecuencia): A y F bajan, y G y J suben menos de un 8 %.
- **Sin regresión:** el Proyecto 3 no encareció el sistema del preproyecto. Sus huellas `LEGACY_A_J` siguen iguales.

## Sistema del Proyecto 3

Son ms por cuadro (mediana de 3 medianas, rango entre paréntesis) y los contadores del último cuadro. Cada cuadro hace 7 pruebas de vista: los cinco cuerpos (la estrella y los cuatro planetas), los lazos de plasma y la nave.

| Escenario | 1 hilo (ms) | 20 hilos (ms) | Dibujos | Triángulos enviados | Objetos descartados | Fragmentos | LOD |
|---|---:|---:|---:|---:|---:|---:|---|
| A spacecraft alone | 5.17 (5.17–5.17) | 7.66 (6.21–8.29) | 4 | 894 | 0 de 1 | 25998 |  |
| B one planet | 25.09 (25.07–25.09) | 10.37 (10.26–10.44) | 1 | 192 | 1 de 2 | 5538 | entrance:- |
| C distant system with three bodies | 9.09 (9.08–9.12) | 6.92 (6.68–7.65) | 47 | 4786 | 0 de 7 | 5221 | crown:-;entrance:-;clover:L2;hearts:L2;house:- |
| D full system with the spacecraft | 23.02 (22.90–23.17) | 15.79 (15.70–17.02) | 46 | 4752 | 0 de 7 | 40043 | crown:-;entrance:-;clover:L2;hearts:L2;house:- |
| E close approach to Crimson Entrance | 315.62 (315.08–336.87) | 38.53 (38.15–40.72) | 24 | 3680 | 2 de 7 | 99575 | crown:-;entrance:-;clover:-;hearts:L2;house:- |
| F close approach to House of Cards | 51.71 (51.70–51.78) | 15.11 (14.91–23.57) | 14 | 1630 | 4 de 7 | 111919 | crown:-;entrance:-;clover:L2;hearts:-;house:- |
| G camera facing empty space | 5.20 (5.19–8.63) | 10.03 (8.72–10.09) | 4 | 894 | 6 de 7 | 25998 | crown:-;entrance:-;clover:-;hearts:-;house:- |
| H full system with the star field | 22.99 (22.82–30.00) | 16.28 (16.13–20.25) | 46 | 4752 | 0 de 7 | 40043 | crown:-;entrance:-;clover:L2;hearts:L2;house:- |
| I full system without the sky (stars and nebula) | 21.29 (21.27–28.74) | 13.72 (12.15–13.90) | 46 | 4752 | 0 de 7 | 40043 | crown:-;entrance:-;clover:L2;hearts:L2;house:- |
| J lowest flight over Crimson Entrance | 678.16 (669.66–693.58) | 74.74 (73.91–76.13) | 24 | 3680 | 2 de 7 | 228224 | crown:-;entrance:-;clover:-;hearts:L2;house:- |
| L1 relief far with automatic LOD | 57.42 (57.24–57.61) | 22.13 (20.68–24.01) | 14 | 2778 | 3 de 7 | 60383 | crown:-;entrance:L2;clover:-;hearts:-;house:- |
| L1F relief far with level 5 forced | 61.73 (61.66–62.03) | 27.67 (19.88–29.67) | 14 | 22938 | 3 de 7 | 60452 | crown:-;entrance:L5;clover:-;hearts:-;house:- |
| L2 relief mid-range with automatic LOD | 532.08 (531.77–532.38) | 65.95 (65.34–68.47) | 5 | 6014 | 5 de 7 | 151672 | crown:-;entrance:L4;clover:-;hearts:-;house:- |
| L2F relief mid-range with level 5 forced | 536.13 (535.71–536.28) | 72.42 (68.70–77.71) | 5 | 21374 | 5 de 7 | 151824 | crown:-;entrance:L5;clover:-;hearts:-;house:- |
| L3 relief grazing with automatic LOD | 738.76 (729.29–765.82) | 82.82 (81.92–84.10) | 24 | 8608 | 2 de 7 | 238522 | crown:-;entrance:L4;clover:-;hearts:L2;house:- |
| L3F relief grazing with level 5 forced | 738.06 (737.59–760.55) | 92.92 (91.74–93.35) | 24 | 44128 | 2 de 7 | 238694 | crown:-;entrance:L5;clover:-;hearts:L5;house:- |
| K close flight over the clover planet | 183.03 (182.90–183.06) | 36.56 (36.36–36.59) | 38 | 17890 | 4 de 7 | 211563 | crown:-;entrance:-;clover:L4;hearts:-;house:- |
| K2 clover planet from afar | 9.57 (9.55–9.59) | 6.91 (6.84–9.75) | 7 | 1746 | 5 de 7 | 31741 | crown:-;entrance:-;clover:L2;hearts:-;house:- |
| K3 clover mountain and waterfall up close | 145.50 (143.81–145.60) | 38.06 (36.91–41.11) | 11 | 26630 | 5 de 7 | 245913 | crown:-;entrance:-;clover:L5;hearts:-;house:- |
| M close flight over the hearts pyramid and city | 227.94 (227.86–233.13) | 47.78 (47.63–68.16) | 43 | 39408 | 5 de 7 | 235336 | crown:-;entrance:-;clover:-;hearts:L4;house:- |
| M2 hearts planet from afar | 34.07 (34.06–41.40) | 13.13 (11.79–18.33) | 25 | 4000 | 1 de 7 | 48031 | crown:-;entrance:-;clover:L2;hearts:L2;house:- |

## Descarte: con y sin él

Mismo cuadro, sin descarte de objetos fuera de la vista ni de sectores bajo el horizonte:

| Escenario | Triángulos con descarte | Sin descarte | Dibujos con / sin | 20 hilos con descarte (ms) | Sin descarte (ms) | 1 hilo con / sin (ms) |
|---|---:|---:|---:|---:|---:|---:|
| E | 3680 | 22150 | 24 / 73 | 38.53 | 41.43 | 315.62 / 318.85 |
| F | 1630 | 40840 | 14 / 76 | 15.11 | 21.56 | 51.71 / 58.13 |
| G | 894 | 57918 | 4 / 102 | 10.03 | 17.14 | 5.20 / 11.77 |
| J | 3680 | 5746 | 24 / 47 | 74.74 | 75.69 | 678.16 / 654.39 |
| K | 17890 | 65496 | 38 / 102 | 36.56 | 45.37 | 183.03 / 190.98 |
| K2 | 1746 | 41514 | 7 / 76 | 6.91 | 13.12 | 9.57 / 15.46 |
| K3 | 26630 | 78078 | 11 / 102 | 38.06 | 43.83 | 145.50 / 150.72 |
| L1 | 2778 | 41642 | 14 / 76 | 22.13 | 20.48 | 57.42 / 62.14 |
| M | 39408 | 58674 | 43 / 102 | 47.78 | 50.21 | 227.94 / 230.31 |
| M2 | 4000 | 4786 | 25 / 47 | 13.13 | 18.86 | 34.07 / 34.29 |

- **El descarte cambia el trabajo del renderizador.**
  - Mirando al vacío (G), los triángulos enviados bajan de 57 918 a 894: solo la nave y su llama. El cuadro baja de 17.1 a 10.0 ms con 20 hilos y de 11.8 a 5.2 ms con 1.
  - Con el trébol lejos (K2): de 41 514 a 1746 triángulos y de 13.1 a 6.9 ms.
- **Cerca de una superficie (E, J, M)** el ahorro de tiempo es pequeño. El costo lo dominan los fragmentos del planeta en pantalla, que se sombrean igual con descarte o sin él.
- **Ruido:** en J con 1 hilo y en L1 con 20, la variante sin descarte sale más rápida (654 frente a 678 ms; 20.5 frente a 22.1 ms) aunque envía más triángulos. Es ruido entre corridas: se dibujan los mismos fragmentos.

## LOD: automático frente al nivel 5 forzado

| Caso | Triángulos automático | Nivel 5 forzado | 1 hilo automático / forzado (ms) | 20 hilos automático / forzado (ms) |
|---|---:|---:|---:|---:|
| L1 | 2778 | 22938 | 57.42 / 61.73 | 22.13 / 27.67 |
| L2 | 6014 | 21374 | 532.08 / 536.13 | 65.95 / 72.42 |
| L3 | 8608 | 44128 | 738.76 / 738.06 | 82.82 / 92.92 |

- **Lejos (L1),** el LOD automático envía 2778 triángulos en lugar de 22 938 y ahorra un 20 % con 20 hilos.
- **Cerca (L2, L3),** el ahorro es de 9–11 % con 20 hilos. Con 1 hilo no se nota: dominan los fragmentos.

## Peor caso y margen

- **Con 20 hilos** (lo que usa la ventana):
  - el peor caso es L3F, 92.9 ms. Es el nivel 5 forzado, solo para comparar;
  - con LOD automático, el peor es L3 (82.8 ms), seguido de J (74.7 ms) y L2 (66.0 ms);
  - **ningún caso pasa de 100 ms** (10 FPS de CPU).
  - Los planetas nuevos de cerca cuestan 36.6 ms (K, el trébol), 38.1 ms (K3, montaña y cascada) y 47.8 ms (M, la pirámide, la ciudad, las naves, las motos y las estelas).
- **Por qué J cuesta más en el sistema del Proyecto 3 (74.7 frente a 42.1 ms):** el reescalado de C23 agranda los planetas frente a la nave. En el vuelo rasante, Crimson Entrance llena más pantalla: 228 224 fragmentos frente a 132 873 (+72 %), con su material del Laboratorio 6 (nubes, sombras y bump por fragmento). Los triángulos solo pasan de 2650 a 3680.
- **Con 1 hilo** se ve el costo puro de CPU: de 5 ms (la nave sola) a 739 ms (L3). Los vuelos rasantes son los que más pesan.

## Memoria

- `peak_rss_kb` es el pico del proceso (`VmHWM`), acumulado y no por escenario. La corrida mezcla los dos sistemas y alterna el orden, así que el pico es el de todo el proceso: de 54.2 a 55.8 MB.
- La línea base de C02 llegaba a 29.9 MB al final de A–J, solo con el sistema del preproyecto. La diferencia la ponen las escenas del Proyecto 3: las mallas de relieve de dos planetas en cuatro niveles, los árboles y rocas, la ciudad y el tráfico. No se midieron por separado.

## Gravedad encendida o apagada

La gravedad no entra en el cuadro de render: es física, con 4 fuentes en cada subpaso de 1/240 s. Se midió aparte el paso completo de simulación: entradas, órbitas, vuelo con gravedad y colisión. Fueron 600 pasos de 1/60 s, 7 rondas, 3 corridas en orden alterno.

| Gravedad | Mediana por paso (µs), corridas 1–3 |
|---|---|
| Encendida | 55.77, 55.66, 55.70 |
| Apagada | 62.11, 55.57, 55.50 |

La diferencia no se distingue del ruido. Un paso de simulación cuesta unos 0.06 ms, menos del 0.4 % de un cuadro de 16.7 ms.

## Veredicto

**PASS.**
- No hay regresión crítica: el sistema del preproyecto queda dentro del ruido de la línea base.
- El peor caso con 20 hilos (92.9 ms, nivel 5 forzado) no pasa de 100 ms.
- El descarte y el LOD reducen los triángulos enviados de forma medible.
