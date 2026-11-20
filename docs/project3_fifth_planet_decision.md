# Proyecto 3 — Decisión del quinto planeta (C70)

**Estado: `ART_PENDING_APPROVAL`.** Ningún concepto está aprobado. El sistema cierra con cuatro planetas (los cuatro palos de la baraja) y el techo de la expansión queda en **190 puntos**. La ranura orbital exterior sigue reservada. Este documento no crea ningún planeta activo.

## Contexto

El menú del proyecto puntúa hasta cinco planetas, sin contar la estrella. Los cuatro actuales son los palos de la baraja:

| Palo | Planeta | Órbita | Rasgo |
|---|---|---|---|
| ♠ | Crimson Entrance | 14 | Corona real y lazos de plasma |
| ♣ | Clover Haven | 21 | Islas de trébol, árboles, rocas y cascadas |
| ♥ | Neon Hearts | 29.5 | Terrazas de neón, ciudad, pirámide, naves y motos |
| ♦ | House of Cards | 40 | Anillo de cartas |

El quinto tiene que ser distinto de los cuatro: no basta con cambiar el color de una esfera. Los dos conceptos reutilizan la icoesfera, el campo de altura, el LOD y los adornos del sistema, sin copiar código.

## Concepto 1 — «Joker / Comodín»

- **Motivo:** carnaval torcido. Es el comodín de la baraja, la carta que no pertenece a ningún palo.
- **Geometría:** un relieve en espiral, una rampa helicoidal que baja del polo al ecuador. Encima, crestas curvas en forma de «sonrisa».
- **Material:** bandas de arlequín púrpura y verde ácido que giran con el tiempo, y rombos.
- **Rasgo propio:** una carpa de circo en el centro y cascabeles que orbitan bajo, cerca de la superficie.
- **Riesgos:**
  - La espiral necesita un campo de altura nuevo (`HeightField`) con su propia cota de relieve (`max_relief`). Afecta al layout y a la colisión.
  - Las bandas animadas cambian cada fotograma, así que hacen falta oráculos con tiempo fijo.
  - Los cascabeles en órbita baja suman cuerpos que la cámara y la nave deben esquivar.
  - Hay que leerlo como «comodín» sin texto ni logotipos: el motivo depende del color y de la forma.

## Concepto 2 — «As de Cristal»

- **Motivo:** hielo facetado. Es el as, la carta más alta.
- **Geometría:** facetas de Voronoi en relieve escalonado, con prismas de cristal como modelos de superficie.
- **Material:** refracción aparente. El color depende del ángulo de cada faceta y hay destellos especulares. No usa transparencia (alfa): el pipeline no la tiene.
- **Rasgo propio:** una corona de agujas de hielo en un polo.
- **Riesgos:**
  - Voronoi por fragmento es caro en CPU. Para mantener el FPS haría falta hornear las celdas en los vértices.
  - Sin alfa, el «cristal» depende solo del sombreado y puede parecer plástico.
  - La corona de agujas recuerda a la corona real de Crimson Entrance. Habría que distinguirlas por color y forma.

## Decisión del usuario

No existe una decisión explícita sobre la identidad. La instrucción de ejecución del Gate (2026-11-20) dice, literalmente:

> «El quinto planeta sigue siendo una decisión artística pendiente. […] Esta instrucción autoriza la ejecución general del Gate, pero NO significa que una identidad artística concreta haya sido aprobada. Cuando llegues a C70: 1. Documenta ambos conceptos conforme al Gate. 2. Conserva ART_PENDING_APPROVAL si todavía no existe una decisión explícita del usuario. 3. No atribuyas al usuario una aprobación que no dio. 4. No ejecutes C71 ni C72 sin esa aprobación.»

Por eso:

- `src/solar/system/spec.rs` gana solo `FIFTH_PLANET = FifthPlanetStatus::ArtPendingApproval`, sin ningún `PlanetSpec` nuevo.
- `SystemSpec::project3()` tiene exactamente cuatro planetas. Lo comprueba `tests/p3_system_spec_tests.rs` (`fifth_absent_while_pending`).
- C71 (terreno y material) y C72 (registro en la órbita exterior) quedan **NO EJECUTADOS POR APROBACIÓN ARTÍSTICA PENDIENTE**.

## Si llega la aprobación

1. Escribir aquí la decisión literal del usuario, con la fecha.
2. Cambiar `FIFTH_PLANET` a `FifthPlanetStatus::Approved("joker")` o `Approved("crystal-ace")`.
3. Ejecutar C71 (campo y material del concepto) y C72 (`PlanetSpec`, ranura orbital de diseño r ≈ 45, tecla F5 y oráculos).
