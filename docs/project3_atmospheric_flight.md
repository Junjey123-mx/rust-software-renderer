# Proyecto 3 — Acople gravitacional, vuelo atmosférico y atmósferas

Este documento empieza el 2026-11-20 con la auditoría del problema, antes de tocar el código, y se completa al final con lo que se hizo.

## 1. Auditoría: qué empuja la nave hacia afuera

El síntoma: al acercarse a un mundo, la nave no se queda cerca de la superficie y la vista tiende a salir hacia afuera. Hay cinco causas en el código, y se suman.

1. **El vuelo local solo sube** (`flight::local_motion`):
   - la distancia al centro se calcula como `(distancia + E/Q).max(suelo + holgura)`;
   - al pasar sobre una montaña, la nave sube para respetar la holgura, pero al bajar la montaña no vuelve a bajar: no hay altura objetivo;
   - cada relieve la empuja un poco más hacia afuera y nunca regresa.
2. **El vuelo espacial choca con una esfera alta** (`SolarScene::collision_shapes`):
   - cerca de un planeta, la nave choca con la esfera del relieve más alto, no con el suelo;
   - en Ace Crystal esa esfera mide 1.46 R, así que la nave se detiene muy por encima de la superficie.
3. **La órbita asistida queda alta** (`capture::CaptureZone::around`):
   - se asienta en `floor + 0.25 R`, donde `floor` ya está por encima del relieve más alto (o del anillo de cartas en House of Cards);
   - `E`/`Q` no pueden bajar de `floor`.
4. **No hay entrada automática a un modo cercano:**
   - la captura exige llegar lento y quedarse 0.25 s;
   - el vuelo local exige pulsar `L` dentro de un rango;
   - una nave que llega rápido no entra a ninguno de los dos, rebota contra la esfera de colisión y la gravedad (a lo más un cuarto del empuje) no la retiene.
5. **La cámara es la del espacio** (`camera::FollowRig`):
   - 0.9 u detrás y 0.22 u arriba en cualquier modo;
   - usa el «arriba» de la nave, que antes de la segunda pasada no se nivelaba con el planeta. La nivelación (`5e99480`) ya lo corrige, pero el encuadre sigue pensado para el espacio.

**Lo que no es causa:**

- el cálculo de la gravedad, que es una inversa del cuadrado ablandada y siempre atrae;
- la proyección tangencial: `local_motion` ya avanza sobre el plano tangente, por arcos de círculo máximo.

## 2. Plan

- **Vuelo atmosférico**, solo en el visor y con el controlador de captura activo, para no cambiar las grabaciones ni las pruebas históricas:
  - mientras dura el vuelo local, una **altura objetivo sobre el suelo** que `E`/`Q` ajustan, entre la holgura y el techo de la atmósfera;
  - un controlador de altura suave (atenuación exponencial, críticamente amortiguado) que lleva la nave a esa altura, hacia arriba y hacia abajo, siguiendo el relieve;
  - la holgura sigue como piso duro;
  - el avance se mantiene sobre el plano tangente (`local_motion`), y la nivelación alinea el «arriba» con la normal y la nariz con la tangente.
- **Entradas y salida:**
  - en vuelo espacial con gravedad, entrar a la atmósfera de un planeta (por debajo de su esfera de colisión más un margen) pasa al vuelo atmosférico sin pedir velocidad baja; la nave baja después con suavidad a la altura de exploración;
  - desde la captura orbital, `Q` en el piso de la órbita o `L` bajan a la atmósfera;
  - `SPACE` (turbo) sale de la atmósfera por la tangente y hacia arriba, como el escape tangencial: bloquea la reentrada hasta alejarse.
- **Cámara atmosférica:** más atrás, más alta y mirando más adelante, con una transición suave entre ese encuadre y el del espacio.
- **Anillo de House of Cards:** decorativo en el vuelo atmosférico, que no tiene colisiones. La nave lo atraviesa y no se rompe el control.
- **Atmósferas visibles:** una pasada en pantalla sobre el búfer de profundidad, con un color por planeta:
  - un halo en el borde del disco, más intenso del lado iluminado;
  - un tinte del borde sobre el disco;
  - dentro de la atmósfera, el cielo se tiñe con su color de día y se oscurece de noche.
