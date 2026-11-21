# Proyecto 3 — Evidencia de gravedad y colisión (C74)

Este documento recoge la evidencia de dos rubros del menú: **atracción gravitatoria** (10 puntos) y **colisión con los planetas** (4 puntos). Las pruebas están en `tests/p3_gravity_collision_integration_tests.rs` y corren sobre el sistema del Proyecto 3 a escala real (cuatro planetas).

```bash
cargo test --release --test p3_gravity_collision_integration_tests
```

## Modelo

- **Gravedad:** cada planeta con `mu` atrae a la nave con el inverso del cuadrado suavizado, `a = Σ mu·(c − p) / (|c − p|² + ε²)^(3/2)`, con `ε = 0.25 R` (`src/solar/physics/gravity.rs`). Los planetas siguen sus órbitas y no se atraen entre sí. La estrella no atrae.
- **`mu` por planeta** (`design_mu`): el menor de dos límites. La atracción en la superficie no pasa de un cuarto del empuje (3 u/s²), y la velocidad de escape no pasa de la mitad del crucero (3 u/s).

| Planeta | R | `mu` | Atracción en la superficie (u/s²) |
|---|---:|---:|---:|
| Crimson Entrance | 1.10 | 0.994 | 0.75 |
| Clover Haven | 1.45 | 1.631 | 0.71 |
| Neon Hearts | 1.30 | 1.388 | 0.75 |
| House of Cards | 1.75 | 1.969 | 0.59 |

- **Tecla `G`:** enciende o apaga la gravedad sin tocar nada más. La barra de título termina en `grav on` o `grav off`, y el HUD (`H`) muestra `GRAV ON` o `GRAV OFF`.
- **Vuelo local (`L`):** ignora la gravedad. El suelo sostiene la nave.
- **Colisión:** barrido del segmento de cada subpaso contra la esfera del planeta, en el punto más alto de su relieve, más el radio de la nave y un margen de 0.05. Se anula la velocidad que entra al planeta, relativa a su movimiento (`src/solar/physics/collision.rs`).

## Trayectoria recta frente a curva

La escena está en pausa (campo estático). La nave pasa por encima de Crimson Entrance, a tres radios del centro, a 1 u/s en horizontal, sin amortiguadores ni empuje. Las coordenadas son relativas al centro del planeta, cada 0.5 s.

Para regenerar el CSV completo (24 filas):

```bash
P3_DUMP_DIR=<dir> cargo test --release --test p3_gravity_collision_integration_tests straight_vs_curved
```

Se escribe en `<dir>/gravity_trajectories.csv`.

| t (s) | G apagada: x | G apagada: y | G encendida: x | G encendida: y | Desvío (u) |
|---:|---:|---:|---:|---:|---:|
| 0.27 | 0.2667 | 3.3000 | 0.2665 | 3.2967 | 0.003 |
| 0.77 | 0.7667 | 3.3000 | 0.7639 | 3.2735 | 0.027 |
| 1.27 | 1.2666 | 3.3000 | 1.2557 | 3.2291 | 0.072 |
| 1.77 | 1.7666 | 3.3000 | 1.7395 | 3.1655 | 0.137 |
| 2.27 | 2.2667 | 3.3000 | 2.2137 | 3.0854 | 0.221 |
| 2.77 | 2.7668 | 3.3000 | 2.6773 | 2.9912 | 0.322 |
| 3.27 | 3.2669 | 3.3000 | 3.1302 | 2.8856 | 0.436 |
| 3.77 | 3.7670 | 3.3000 | 3.5724 | 2.7707 | 0.564 |
| 4.27 | 4.2671 | 3.3000 | 4.0046 | 2.6483 | 0.703 |
| 4.77 | 4.7672 | 3.3000 | 4.4271 | 2.5199 | 0.851 |
| 5.27 | 5.2673 | 3.3000 | 4.8409 | 2.3867 | 1.008 |
| 5.77 | 5.7674 | 3.3000 | 5.2464 | 2.2497 | 1.172 |

- **Con la gravedad apagada,** la nave sigue la recta (desvío de la línea < 1e-4).
- **Con la gravedad encendida,** en 5.8 s baja 1.05 u, casi un radio (R = 1.10), y se queda 0.52 u atrás. La curva se ve a simple vista y la nave nunca entra al planeta.
- **Caso negativo:** con la gravedad encendida pero `mu` = 0 en todos los cuerpos, la trayectoria es la misma recta (±1e-4).
- **Captura superpuesta:** los dos cuadros, sin gravedad y con ella, se superponen en `docs/screenshots/project3/05-gravity-on-off.png` (C78).

## Las pruebas

| Prueba | Qué comprueba |
|---|---|
| `straight_vs_curved` | Recta exacta sin G; con G, curva hacia el planeta de más de 0.5 R sin entrar en él; con `mu` = 0, la misma recta |
| `two_planet_superposition` | Entre Crimson Entrance y el trébol, la atracción de los dos es la suma de cada una, y cada término apunta a su centro. Las cuatro fuentes suman lo mismo que por separado |
| `center_softening` | En el centro la atracción es 0 y finita. Nunca pasa del máximo teórico `2/(3√3)·mu/ε²`, y lejos (40 ε) es el inverso del cuadrado con un error < 1 % |
| `boost_into_planet_no_penetration` | Turbo directo contra cada uno de los cuatro planetas, con G encendida, a 60 FPS y a 10 FPS |
| `local_and_warp_safe` | Después del viaje rápido a cada planeta (F1–F4), turbo contra él. En vuelo local sobre las terrazas de Neon Hearts, bajando, la nave nunca queda bajo el suelo y la ruta es la misma con G o sin ella |
| `fps_equivalence` | La trayectoria curva de 4 s a 30 y a 144 FPS queda a menos del 1 % de la de 60 FPS (paso físico fijo de 1/240 s) |
| `hud_shows_gravity_state` | `G` da «Gravity off» o «Gravity on», la barra de título cambia a `grav off` o `grav on`, y sin gravedad no hay fuentes |

## Colisión: distancia mínima medida

Es la menor distancia entre la esfera de la nave y el punto más alto del relieve, medida contra la posición del planeta al final de cada cuadro, durante 5 s de turbo directo hacia él:

| Planeta | 60 FPS | 10 FPS | Avance del planeta en un cuadro a 10 FPS |
|---|---:|---:|---:|
| Crimson Entrance | +0.066 | +0.141 | 0.100 |
| Clover Haven | +0.035 | −0.035 | 0.090 |
| Neon Hearts | +0.041 | −0.007 | 0.085 |
| House of Cards | +0.037 | −0.025 | 0.075 |

- **A 60 FPS** la nave nunca toca el planeta: queda sobre el margen de 0.05.
- **A 10 FPS**, mientras la nave resbala por el costado de un planeta en órbita, puede quedar hasta 0.035 u (2–3 % de R) dentro de donde el planeta está al final del cuadro.
  - La causa es que la forma de colisión adelanta al planeta hasta un cuadro de su órbita. La prueba admite como cota ese avance, |v_planeta|·dt, igual que `p3_collision_sweep_tests`.
  - Nunca atraviesa el planeta, ni siquiera a 10 FPS.
  - **Limitación conocida:** para eliminar ese solape habría que construir las formas de colisión con la posición del planeta al inicio del cuadro (`src/solar/viewer.rs`). Queda fuera de este commit, que es solo de pruebas.

## Corrección de la colisión a FPS bajos (2026-11-20)

La limitación de arriba ya está corregida. La tabla anterior se conserva como historial.

- **Causa:** `SolarScene::collision_shapes` construía cada forma con la posición del planeta al **final** del cuadro (el reloj ya lo había avanzado), y `physics::collide` la movía además por su velocidad durante los subpasos. La forma iba un cuadro de órbita por delante del planeta.
- **Corrección:** la forma arranca en `posición − v_planeta·real_delta`, donde estaba el planeta al inicio del cuadro. En cada subpaso, nave y planeta se comparan en el mismo instante, con la velocidad relativa.
- **Pruebas:** `tests/p3_collision_moving_target_tests.rs`:
  - turbo radial contra los cinco planetas;
  - un planeta que alcanza a la nave detenida en su órbita (blanco móvil);
  - la regresión de Clover, Hearts y House;
  - todo a 10, 15, 30, 60 y 144 FPS.

  `boost_into_planet_no_penetration` exige ahora el margen, en vez de admitir el avance de un cuadro.

Distancia mínima medida después de la corrección: la esfera de la nave contra el punto más alto del relieve, con el planeta al final de cada cuadro, durante 5 s de turbo directo hacia él.

| Planeta | 60 FPS | 10 FPS |
|---|---:|---:|
| Crimson Entrance | +0.050 | +0.050 |
| Clover Haven | +0.050 | +0.050 |
| Neon Hearts | +0.050 | +0.050 |
| House of Cards | +0.050 | +0.050 |
| Joker | +0.050 | +0.050 |

House of Cards queda en +0.0496 y el Joker a 60 FPS en +0.0497; los demás, en +0.0500. Ningún planeta, a ningún FPS, deja a la nave dentro de su forma.
