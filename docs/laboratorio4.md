# Laboratorio 4 — Carga y dibujo de un modelo OBJ

Guía técnica del trabajo implementado en la rama `lab4/obj-wireframe`.

## Objetivo

Cargar un archivo OBJ triangulado, recorrer todos sus triángulos, recuperar los
tres vértices de cada cara mediante sus índices y dibujar cada triángulo con la
función `triangle`, que a su vez usa `line`. El resultado se muestra centrado,
con tamaño razonable y completamente visible en una ventana de 800×600.

## Modelo de entrada

- Ruta: `assets/models/Laboratorio3.obj` (objeto `NaveBimotor`, exportado desde
  Blender ya triangulado). El archivo se copió sin modificar su geometría.
- Registros que usa el cargador: `v x y z` (posiciones) y `f a b c` (caras).
- Registros ignorados: `vt`, `vn`, `mtllib`, `usemtl`, `o`, `s`, `g` y
  comentarios. El archivo `.mtl` referenciado no es necesario.
- Esquinas de cara admitidas: `i`, `i/t`, `i//n`, `i/t/n`; solo se lee el
  número anterior al primer `/`.

Valores que el cargador obtiene de este archivo (verificados por tests):

| Dato | Valor |
|---|---:|
| posiciones `v` | 356 |
| caras `f` | 708 |
| referencias de índice | 2124 |
| rango X | [-2.83, 2.83] |
| rango Y | [-0.42, 0.93] |
| rango Z | [-4.4, 1.8] |

## Conversión de índices

OBJ numera las posiciones desde 1; `Vec` en Rust desde 0. El cargador:

1. parsea el token como entero sin signo;
2. rechaza `0` y valores negativos o no numéricos (`InvalidIndex`, con línea);
3. resta 1 **solo después** de esa validación;
4. al terminar el archivo comprueba que cada índice sea menor que el número de
   posiciones (`IndexOutOfRange`, con línea).

Una cara con más o menos de tres esquinas se rechaza (`NonTriangularFace`); no
se descarta silenciosamente el cuarto índice.

## Representación en memoria

```text
Mesh { vertices: Vec<Vector3>, indices: Vec<usize> }   // compacta, indexada
       │
       └─ expand_vertices(color) → Vec<Vertex>          // una entrada por esquina
                                                        // de cara, en orden
Vertex { position, transformed_position, color }
```

`Mesh::new` valida: índices dentro de rango, cantidad múltiplo de tres y al
menos una cara. Para la nave, `expand_vertices` produce 2124 `Vertex`; agrupados
de tres en tres son los 708 triángulos originales. Nunca se agrupan las 356
posiciones únicas por ternas: eso no corresponde a las caras del archivo.

## Pipeline (cuatro etapas)

```text
vertex_shader (matriz de modelo, identidad) + fit_to_viewport (mapeo a píxeles)
    → primitive_assembly   (ternas consecutivas → [Vertex; 3])
    → rasterize            (triangle → 3 × line → Vec<Fragment>)
    → fragment_shader      (passthrough) → framebuffer.point
```

- `vertex_shader` multiplica `(x, y, z, 1)` por la matriz 4×4 de `Uniforms`
  (campos `m0..m15` de raylib) y actualiza solo `transformed_position`.
- `fit_to_viewport` se aplica una sola vez, antes de ensamblar; `triangle` y
  `line` leen posiciones ya en píxeles y no vuelven a escalar.
- `rasterize` no escribe píxeles: devuelve fragmentos. La escritura ocurre en la
  cuarta etapa mediante `Framebuffer::point`, que descarta coordenadas fuera de
  la imagen sin fallar.
- `render` devuelve `RenderStats` con los conteos reales de la ejecución:
  2124 vértices, 708 triángulos y 79 934 fragmentos para la nave en 800×600.

## Líneas y triángulos propios

- `line(start, end, color)`: Bresenham entero para todos los octantes, ambos
  extremos incluidos, coordenadas redondeadas con `.round() as i32`.
- `triangle(a, b, c)`: exactamente `line(a,b)`, `line(b,c)`, `line(c,a)`. Sin
  relleno.

## Vista plana y encuadre

No hay perspectiva ni cámara. Se eligen dos ejes del modelo como plano de
pantalla (`ViewPlane::XZ` por defecto, vista superior; `XY` disponible), se
calcula el bounding box proyectado y se aplica:

```text
scale    = min(ancho_disponible / span_u, alto_disponible / span_v)
screen_x = ancho/2 + (u - centro_u) * scale
screen_y = alto/2  - (v - centro_v) * scale     // y de pantalla crece hacia abajo
```

con un margen del 10 % por lado. La escala es uniforme (se conserva la relación
de aspecto) y el centro del modelo coincide con el centro de la ventana. Los
casos degenerados (span cero) no dividen por cero.

## Presentación

Raylib solo crea la ventana y dibuja **una textura** rellenada con los bytes
RGBA que produce `Framebuffer::to_rgba_bytes`. No se usan `LoadModel`,
`DrawModel`, `DrawMesh`, `DrawLine` ni `DrawTriangle`.

## Límites de esta etapa

Solo contornos: sin relleno, sin eliminación de caras ocultas (las aristas
traseras se superponen a las delanteras), sin z-buffer, sin texturas, sin
iluminación y sin proyección en perspectiva. El relleno de triángulos
corresponde al Laboratorio 5, en otra rama.
