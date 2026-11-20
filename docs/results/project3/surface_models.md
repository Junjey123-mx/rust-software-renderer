# Proyecto 3 — Modelos sobre la superficie de los planetas (P3-07)

El sistema tiene cuatro tipos distintos de modelos colocados sobre el suelo de un planeta. Los cuatro son geometría procedural propia, escrita en este repositorio, sin archivos OBJ externos. Cada uno se hornea una sola vez al cargar la escena, en el espacio del planeta, y se dibuja con la matriz del planeta, así que gira con él.

## Los cuatro tipos

| Tipo | Planeta | Código | Triángulos por modelo | Alto / ancho | Colocados (semilla por defecto) |
|---|---|---|---|---|---|
| Árbol | Clover Haven | `tree_mesh` / `tree_parts` en `src/solar/attach/prop_meshes.rs` | 44 a 56 (tres variantes) | 1.37 a 2.13 | 225 |
| Roca | Clover Haven | `rock_mesh` en `src/solar/attach/prop_meshes.rs` | 80 | 0.62 | 56 |
| Edificio neón | Neon Hearts | `tower_mesh` en `src/solar/attach/prop_meshes.rs` | 56 a 84 con sus bordes (tres variantes) | 0.53 a 4.13 | 435 |
| Pirámide escalonada | Neon Hearts | `pyramid_mesh` en `src/solar/attach/prop_meshes.rs` y `Landmark::pyramid` en `src/solar/attach/landmark.rs` | 140 con sus bordes, más 160 del disco del corazón | 0.98 | 1 |

**Qué tiene cada uno:**
- **Árbol:** tronco hexagonal y dos o tres conos de copa. El tronco y la copa son capas de color distintas.
- **Roca:** icosfera de nivel 1, deformada por la semilla y aplastada al 60 % de su alto.
- **Edificio neón:** pisos de caja (torre escalonada, bloque cívico bajo o aguja) con tiras de neón en las aristas.
- **Pirámide:** cinco escalones cuadrados de 1 a 0.16 de ancho, con neón en cada escalón. Encima flota el disco del corazón, que gira sobre la normal del suelo. El disco es parte del mismo hito y no se cuenta como un quinto tipo.

**Colocación:**
- Los árboles y las rocas se dispersan sobre tierra firme del trébol (nunca bajo el mar).
- Los edificios se dispersan solo sobre mesetas de las terrazas, a más de 0.3 rad del pie de la pirámide.
- La pirámide se mueve a la meseta más cercana a su dirección.

Los números de la tabla salen de `tests/p3_surface_model_rubric_tests.rs` con la semilla 7 del trébol y la 11 de Hearts.

## Capturas

| Planeta | Captura | Qué se ve |
|---|---|---|
| Clover Haven (escenario K) | [k_close_flight.png](clover/k_close_flight.png) | Árboles y rocas sobre los lóbulos verdes |
| Neon Hearts (escenario M) | [m_close_flight.png](hearts/m_close_flight.png) | Edificios neón sobre las terrazas, la pirámide y el disco del corazón |

Las dos capturas son cuadros del benchmark escalado a 800×600, con t = 6 s. Son los mismos cuadros cuyas huellas fijan `P3_CLOVER_K` y `P3_HEARTS_M` en `tests/p3_frame_oracle_tests.rs`. Para reproducirlas:

```sh
P3_DUMP_DIR=/tmp/p3 cargo test --test p3_frame_oracle_tests p3_
# escribe /tmp/p3/p3_scaled_K.ppm y /tmp/p3/p3_scaled_M.ppm, entre otros
```

En la ventana: `cargo run --release -- --scene solar`, luego F2 (viaje rápido al trébol) o F3 (a Hearts) y L (vuelo local).

## Pruebas (`tests/p3_surface_model_rubric_tests.rs`)

| Prueba | Qué comprueba |
|---|---|
| `four_distinct_kinds_on_planets` | Hay cuatro tipos. Cada uno está sobre el suelo de su planeta: ningún vértice se hunde más de 0.03 R (0.1 R la pirámide) ni sube más de 0.12 R (0.5 R la pirámide), y alguno toca el suelo. Los cuatro difieren entre sí en triángulos y en proporción alto/ancho. |
| `each_rotates_with_host` | En t = 0, 10 y 25 s, los cuatro tipos se dibujan con la matriz de su planeta, que cambia al girar. El disco del corazón sigue la cima de la pirámide. |
| `crown_and_cards_not_counted` | La corona va sobre la estrella y el anillo de cartas sobre House of Cards; ninguno de los dos tiene superficie propia. Solo cuentan los modelos colocados sobre un planeta con relieve. |

**Caso negativo:** la misma pirámide levantada 0.3 R sobre el suelo ya no cuenta como colocada en la superficie, y la prueba lo detecta.

## Qué no se cuenta

- **La corona real** (`src/solar/royal_crown.rs`): va sobre la estrella, que no es un planeta.
- **Las cartas de diamante** (`src/solar/card_ring.rs`): orbitan House of Cards, no están sobre su superficie.
- **Las variantes de un mismo tipo:** las tres formas de árbol y las tres de edificio cuentan una sola vez cada una.
- **Las cascadas del trébol:** son agua animada, no un modelo.
- **La moto de luz** (C66): queda como quinto tipo de reserva.

## Validación

- `cargo test --test p3_surface_model_rubric_tests`: tres pruebas en verde.
- El G2 completo (fmt, check, clippy con `-D warnings`, todas las suites y `git diff --check`) pasó antes del commit.
