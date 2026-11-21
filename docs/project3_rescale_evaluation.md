# Proyecto 3 — Evaluación del reescalado del universo

**Decisión (2026-11-20): no se aplica en esta entrega.** El sistema conserva las proporciones actuales: planetas de 3 a 6 radios de nave y estrella de 3 a 5 radios del planeta mayor. Este documento registra la evaluación de la propuesta para hacer los cuerpos «considerablemente mayores» y la deja lista para un cambio controlado de `ScaleProfile`.

## Propuesta evaluada

- **Planetas:** de 6 a 10 radios de nave, en lugar de 3–6.
- **Estrella:** de 4 a 6 radios del planeta mayor, en lugar de 3–5.
- **Nave:** igual (radio envolvente 0.319 u) como referencia de escala.

## Método

Las cifras de la tabla son **estimaciones calculadas**: no se renderizaron ni se midieron. Se obtienen así:

1. Se parte de los radios efectivos actuales de cada cuerpo (`SolarScene::shell_of`, que incluye relieve, cascadas, ciudad, tráfico, anillo de cartas, corona y lazos de plasma). Hoy valen:
   - Crimson Entrance 1.10 u, Clover Haven 2.10 u, Neon Hearts 2.31 u, House of Cards 3.74 u y Joker 1.60 u;
   - la estrella, 9.51 u.
2. Esos radios se escalan linealmente: todos los adjuntos están definidos en radios del cuerpo.
3. Se aplica la misma regla de `OrbitLayout::solve`: `r_i = ceil_half(borde + 3 + ef_i)`, con 3 u entre cáscaras vecinas.

Con el factor 1 la regla da las órbitas mínimas 14, 20.5, 28, 37.5 y 46 u. Las órbitas reales (14, 21, 29.5, 40 y 49 u) son valores explícitos que cumplen esos mínimos.

| Planetas × | Estrella | R planeta / nave | Radio estrella (u) | Órbitas mínimas (u) | Radio del mundo (u) | Cruzar el sistema a crucero / a la luz | Periodo del Joker a 1 u/s |
|---|---|---|---:|---|---:|---|---:|
| 1 (actual) | 5.40 u | 3.45–5.48 | 5.40 | 14, 20.5, 28, 37.5, 46 | 52.6 | 35 s / 4.4 s | 289 s |
| 1.8 | 4 × mayor | 6.20–9.87 | 12.60 | 27.5, 36.5, 47.5, 61.5, 74.5 | 82.4 | 55 s / 6.9 s | 468 s |
| 1.8 | 5 × mayor | 6.20–9.87 | 15.75 | 33, 42, 53, 67, 80 | 87.9 | 59 s / 7.3 s | 503 s |
| 1.8 | 6 × mayor | 6.20–9.87 | 18.90 | 38.5, 47.5, 58.5, 72.5, 85.5 | 93.4 | 62 s / 7.8 s | 537 s |

## Consecuencias revisadas

- **Distancias y tiempos:**
  - el mundo crece de 53 u a 82–93 u de radio;
  - cruzar el sistema a 3 u/s pasa de ~35 s a ~1 min, y a la luz (×8) sigue en menos de 8 s;
  - para no alargar la navegación habría que subir `cruise_speed`, o reservar más el viaje rápido.
- **Velocidades orbitales:** con el límite de 1 u/s (la nave debe poder alcanzar cada planeta), los periodos crecen de 4.8 a 7.8–9 min. El cielo y los planetas se verían más quietos.
- **Escala relativa:** las proporciones relativas al radio no cambian:
  - la holgura (0.15 R), el rango local (2.5 R) y la llegada del viaje rápido (1.6 R);
  - las cáscaras de captura y suelta (`CaptureZone`), el anillo de cartas (2.13 R) y la corona;
  - la vuelta local, que se mantiene en ~20 s porque su velocidad es proporcional al radio.
- **Cámara:** la distancia Follow es fija respecto a la nave, así que un planeta de 6–10 radios de nave ocupa mucho más cuadro. Los encuadres de los escenarios A–N y del recorrido grabado tendrían que recalcularse.
- **Rendimiento:** el nivel de detalle se elige por radio proyectado, así que los triángulos por planeta siguen acotados (máx. 20 480). En cambio, un planeta más grande en pantalla sombrea más fragmentos a la misma distancia, y en las vistas rasantes, las más caras (82.8–92.9 ms, [benchmark_final.md](results/project3/benchmark_final.md)), el costo subiría. No se midió.
- **Gravedad:** `mu` de diseño escala con R² (con tope por la velocidad de escape), así que la atracción en la superficie sigue sin pasar de un cuarto del empuje.

## Por qué no se aplica ahora

- **Oráculos:** cambiaría todos los oráculos de cuadro del Proyecto 3, el punto de aparición y el recorrido grabado y sus pruebas.
- **Encuadres:** habría que recalcular los encuadres de los escenarios.
- **Tiempo:** no hay margen para validarlo a horas de la entrega, y no suma puntos de la rúbrica.

## Cómo aplicarlo después

1. Cambiar `ScaleProfile::project3` (`planet_to_ship`, `star_to_planet` y, si se quiere el mismo ritmo, `cruise_speed`).
2. Escalar los radios de `SystemSpec::project3` y fijar las órbitas en `AUTO_ORBIT`, o en valores explícitos que cumplan los mínimos de la tabla.
3. Recalcular los encuadres de `benchmark/solar.rs` y regrabar el recorrido.
4. Volver a capturar los oráculos `P3_*` con el motivo registrado.
