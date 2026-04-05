# Articulo Tecnico: SPH, Fisica y Arquitectura del Proyecto

## 1. Resumen

Este proyecto implementa un simulador de fluidos 2D en Rust usando `nannou`
para visualizacion y un solver `WCSPH` para la dinamica. `WCSPH` significa
`Weakly Compressible Smoothed Particle Hydrodynamics`: un metodo numerico
lagrangiano donde el fluido se representa como un conjunto de particulas que
transportan masa, velocidad, densidad y presion.

La idea central es sencilla:

- En vez de resolver el fluido sobre una malla fija, seguimos particulas.
- Cada particula interactua con sus vecinas dentro de un radio de soporte.
- Las ecuaciones del continuo se aproximan mediante kernels de suavizado.
- El resultado visual es especialmente bueno para superficies libres,
  salpicaduras, mezcla y movimiento de volumenes de liquido.

Este documento explica dos cosas a la vez:

1. La base cientifica y computacional del metodo SPH.
2. Como esa teoria aparece implementada en este codigo concreto.

## 2. Que es SPH

`SPH` es un metodo de simulacion de fluidos y medios continuos basado en
particulas. Nacio en astrofisica, pero hoy se usa en fluidos, graficos por
computador, mecanica de solidos y simulacion interactiva.

Hay dos familias conceptuales muy conocidas en simulacion de fluidos:

- Enfoque euleriano:
  El fluido se calcula sobre una rejilla fija. Lo que importa es que pasa en
  cada celda del espacio.
- Enfoque lagrangiano:
  El fluido se sigue materialmente. Lo que importa es como se mueve cada porcion
  de materia.

SPH es un metodo lagrangiano. Cada particula representa una pequena porcion de
fluido y lleva consigo propiedades del campo:

- posicion
- velocidad
- densidad
- presion
- masa

Una propiedad clave de SPH es que no necesita una malla geometrica explicita
para el dominio interno del fluido. Eso hace que sea muy natural para:

- superficies libres
- gotas y chorros
- fragmentacion
- mezclas visuales
- interaccion con recipientes

## 3. La base fisica: del continuo al metodo de particulas

### 3.1. Ecuaciones del continuo

El comportamiento de un fluido newtoniano suele modelarse con las ecuaciones de
Navier-Stokes y la ecuacion de continuidad.

Forma conceptual:

```text
Continuidad:
d rho / dt + rho * div(v) = 0

Cantidad de movimiento:
rho * dv/dt = -grad(p) + mu * nabla^2(v) + f_ext
```

Donde:

- `rho` es la densidad
- `v` es la velocidad
- `p` es la presion
- `mu` representa viscosidad
- `f_ext` incluye gravedad y otras fuerzas externas

Estas ecuaciones describen el fluido como un medio continuo. El trabajo de SPH
consiste en discretizar estos campos y operadores sobre particulas.

### 3.2. Suavizado por kernels

SPH reemplaza derivadas y campos continuos por sumas ponderadas sobre vecinos.
La herramienta matematica clave es el kernel de suavizado `W(r, h)`, donde:

- `r` es la distancia entre dos particulas
- `h` es el radio de soporte o smoothing radius

La intuicion es:

- una particula influye mucho en vecinas cercanas
- influye poco en vecinas lejanas
- fuera de `h`, la influencia es cero

Eso convierte el problema local en una suma sobre vecinos proximos.

### 3.3. Densidad en SPH

Una de las formulas fundamentales del metodo es:

```text
rho_i = sum_j m_j * W(x_i - x_j, h)
```

Interpretacion:

- `rho_i` es la densidad de la particula `i`
- `m_j` es la masa de la particula `j`
- `W` pondera la contribucion de cada vecina

En este proyecto la densidad se calcula con un kernel `Poly6`, implementado en
`src/sph.rs` dentro de `KernelSet`.

### 3.4. Presion y ecuacion de estado

El proyecto usa un enfoque `WCSPH`. Eso significa que el liquido no se trata
como estrictamente incomprensible, sino como ligeramente compresible. La
presion se obtiene con una ecuacion de estado tipo Tait:

```text
p_i = k * ((rho_i / rho_0)^gamma - 1)
```

Donde:

- `rho_0` es la densidad de reposo
- `k` es la rigidez efectiva
- `gamma` controla la no linealidad

En el codigo:

- `rest_density` es `rho_0`
- `pressure_stiffness` es la rigidez
- `gamma` es el exponente de Tait

Este modelo es comun en `WCSPH` porque evita resolver un sistema global de
presion como hacen otros esquemas mas estrictamente incomprensibles.

### 3.5. Fuerza de presion

Una discretizacion SPH habitual para la aceleracion por presion es:

```text
a_i^pressure = - sum_j m_j *
               (p_i / rho_i^2 + p_j / rho_j^2) *
               grad(W_ij)
```

Esa forma es importante porque:

- es simetrica
- reduce sesgos numericos
- ayuda a respetar la accion y reaccion entre particulas

En tu proyecto aparece en `compute_accelerations`, usando el gradiente del
kernel `Spiky`.

### 3.6. Viscosidad

La viscosidad modela difusion de cantidad de movimiento entre particulas. En
SPH suele escribirse como algo de la forma:

```text
a_i^visc = nu * sum_j m_j * (v_j - v_i) / rho_j * laplacian(W_ij)
```

Esto hace que velocidades muy distintas entre vecinas tiendan a suavizarse. En
la implementacion actual, la viscosidad usa el laplaciano del kernel y aparece
tambien en `compute_accelerations`.

### 3.7. Tension superficial

La tension superficial aparece en fluidos con interfaz libre. Una forma comun
de modelarla en SPH es a traves del campo de color y su gradiente:

```text
n_i = sum_j m_j / rho_j * grad(W_ij)
```

Si el modulo del normal `|n_i|` supera un umbral, se considera que la particula
esta cerca de una superficie libre. Entonces puede aplicarse una fuerza
proporcional a la curvatura local.

En este proyecto:

- se estima un normal superficial
- se usa un laplaciano del campo de color
- se aplica la fuerza solo si el normal supera `surface_threshold`

### 3.8. Gravedad y fuerzas externas

La gravedad se suma como aceleracion constante:

```text
a_gravity = (0, -9.81)
```

La interaccion con el raton tambien se modela como una fuerza externa, pero es
importante remarcar esto:

- la gravedad si es una fuerza fisica real del modelo
- la fuerza del raton no es una fuerza fisica del fluido
- es una fuerza de control interactivo anadida para jugar con la simulacion

## 4. Que significa que este solver sea WCSPH

`WCSPH` es un compromiso entre fidelidad fisica, estabilidad y coste
computacional.

Ventajas:

- relativamente simple
- facil de paralelizar
- muy natural para simulacion visual de liquidos
- evita resolver sistemas lineales globales caros

Desventajas:

- introduce compresibilidad artificial
- puede generar oscilaciones de presion
- necesita pasos pequenos para estabilidad
- puede explotar si las fuerzas se disparan

Por eso tu codigo combina ecuaciones fisicas con varias capas de ingenieria
numerica:

- limite de densidad efectiva para la presion
- limite de aceleracion
- limite de velocidad
- damping suave
- limite especifico de fuerza del raton
- paso temporal fijo y acotado

Es decir: no es solo fisica continua; es fisica discretizada mas controles de
estabilidad.

## 5. Arquitectura del proyecto

La estructura del proyecto es muy pequena y clara:

- `src/main.rs`
  Orquesta la aplicacion, la ventana, el bucle de tiempo, la entrada del raton
  y el render.
- `src/sph.rs`
  Contiene el solver SPH, sus kernels, la estructura espacial, la integracion y
  las estadisticas.
- `Cargo.toml`
  Declara dependencias y opciones de compilacion.

## 6. Como esta modelado el fluido en este codigo

### 6.1. Representacion de datos

El solver no usa un `Vec<Particle>` tradicional. En su lugar utiliza una
organizacion de tipo `Structure of Arrays`:

- `positions: Vec<Vec2>`
- `velocities: Vec<Vec2>`
- `densities: Vec<f32>`
- `inv_densities: Vec<f32>`
- `pressures: Vec<f32>`
- `pressure_terms: Vec<f32>`
- `accelerations: Vec<Vec2>`
- `xsph_corrections: Vec<Vec2>`

Esto es una decision de ciencias de la computacion importante:

- mejora localidad de memoria
- simplifica paralelizacion por columnas de datos
- evita cargar campos que no hacen falta en cada fase
- encaja mejor con bucles numericos masivos

### 6.2. Parametros fisicos y numericos

Los parametros viven en `SphConfig`. Ahí se fija:

- el espaciado de particulas
- el radio de suavizado
- la densidad de reposo
- la rigidez de presion
- la viscosidad
- la tension superficial
- parametros de frontera
- parametros de estabilidad
- parametros de interaccion del raton

Desde el punto de vista de ingenieria, `SphConfig` es el contrato entre:

- el modelo fisico
- la estabilidad numerica
- el comportamiento visual

### 6.3. Kernels

`KernelSet` contiene tres piezas:

- `poly6`
  para densidad y mezcla suave
- `spiky_gradient`
  para fuerza de presion
- `viscosity_laplacian`
  para viscosidad y terminos auxiliares

En SPH el kernel importa mucho porque determina:

- suavidad del campo
- soporte espacial
- estabilidad
- calidad del gradiente

## 7. Busqueda de vecinos: la parte clave de rendimiento

### 7.1. El problema computacional

Si cada particula comprobara su distancia con todas las demas, el coste seria:

```text
O(n^2)
```

Eso se vuelve inviable rapidamente.

### 7.2. La solucion del proyecto

Tu codigo usa una grilla uniforme (`UniformGrid`) para reducir la busqueda a
vecinas locales.

La idea:

- el espacio se divide en celdas
- cada particula se asigna a una celda
- para una particula dada, solo se revisan su celda y las celdas vecinas

Con eso, el coste medio pasa a parecerse mucho mas a:

```text
O(n * k)
```

donde `k` es el numero medio de vecinos dentro del soporte.

### 7.3. Por que esta implementacion es rapida

La grilla actual no es un `Vec<Vec<usize>>` ingenuo. Usa una representacion
compacta:

- `cell_counts`
- `cell_offsets`
- `sorted_particles`
- `particle_cells`
- tablas precalculadas de celdas vecinas

Esto se parece a una estructura tipo `compressed sparse layout` muy sencilla:

1. Cuenta cuantas particulas hay por celda.
2. Calcula offsets prefix-sum.
3. Ordena indices de particulas por celda.
4. Itera sobre rangos contiguos en memoria.

Es una decision muy buena desde CS porque:

- reduce asignaciones dinamicas
- mejora cache locality
- disminuye overhead por celda
- hace la iteracion de vecinos mas predecible

## 8. Paralelismo

El solver usa `rayon`. La paralelizacion aparece en operaciones por particula:

- densidad
- presion
- aceleraciones
- integracion
- calculo de estadisticas

Esta estrategia funciona porque esas etapas son mayormente `data parallel`.
Cada hilo procesa un subconjunto de particulas y solo necesita leer datos
compartidos.

Por que no ves siempre todos los cores al 100%:

- no todo el frame es solver
- el render de `nannou` tiene partes mas seriales
- el numero de particulas no es enorme
- el coste puede estar dividido entre CPU y GPU
- el scheduler no siempre reparte perfecto con cargas cortas

En otras palabras: paralelizar no garantiza ocupacion completa de todos los
nucleos en todo momento.

## 9. El pipeline temporal del solver

Cada paso de simulacion hace lo siguiente:

1. Reconstruir la grilla espacial.
2. Calcular densidades.
3. Calcular presiones.
4. Calcular aceleraciones.
5. Integrar velocidades y posiciones.
6. Actualizar estadisticas si hace falta.

Esto corresponde bastante bien al flujo numerico clasico de un solver SPH
explicito.

## 10. Que hace `src/main.rs`

`src/main.rs` no contiene la fisica del fluido. Contiene la orquestacion.

### 10.1. Ventana y escena

Se crea una ventana fija de `900x900` y una escala `PIXELS_PER_METER` para
convertir el dominio fisico a espacio de pantalla.

### 10.2. Estado inicial

Se inicializa un bloque de particulas:

- `INITIAL_PARTICLES_X = 36`
- `INITIAL_PARTICLES_Y = 48`

Eso produce `1728` particulas en la configuracion actual.

### 10.3. Paso fijo

El proyecto usa:

```text
FIXED_TIME_STEP = 1 / 240
```

Esto es muy importante. En simulacion numerica, un paso fijo:

- hace el solver mas estable
- evita que la fisica dependa demasiado del framerate
- permite acotar error temporal

Tambien hay un limite de pasos por frame:

```text
MAX_STEPS_PER_FRAME = 6
```

Eso significa que, si el render cae mucho, el programa prefiere perder tiempo
simulado antes que intentar recuperar indefinidamente y entrar en espiral de
lag.

### 10.4. Interaccion con el raton

`main.rs` traduce el raton a una estructura `Interaction`:

- posicion
- radio
- intensidad
- modo atraer o repeler

Fisicamente esto no representa una herramienta real del mundo, pero
computacionalmente es una fuerza externa localizada.

### 10.5. Render

El render:

- pinta el contenedor
- dibuja cada particula como una elipse pequena
- colorea por velocidad y densidad relativa
- dibuja un anillo de influencia del raton
- muestra el HUD con FPS y estado del solver

El color no es una variable fisica del modelo. Es una visualizacion derivada de:

- velocidad
- densidad relativa

## 11. Que hace `src/sph.rs`

### 11.1. `SphSimulation`

Es el centro del motor. Contiene:

- parametros
- buffers fisicos
- grilla espacial
- estadisticas

Su metodo principal es `step(dt, interaction)`.

### 11.2. `compute_densities`

Aplica la formula de sumatorio SPH sobre vecinos. Es el equivalente discreto de
reconstruir el campo de densidad local del continuo.

### 11.3. `compute_pressures`

Transforma densidad en presion via la ecuacion de estado de Tait.

Ademas, la implementacion incluye un `clamp` sobre la relacion
`rho / rho_0` para evitar explosiones numericas. Esto no cambia la teoria base
de SPH, pero si modifica su comportamiento numerico para mantener la demo bajo
control.

### 11.4. `compute_accelerations`

Suma varias contribuciones:

- gravedad
- presion
- viscosidad
- tension superficial
- empuje del raton
- fuerzas de borde

Esta funcion es, conceptualmente, la discretizacion de la ecuacion de cantidad
de movimiento.

### 11.5. `integrate`

Actualiza:

- velocidad
- posicion

La integracion es explicita. Es rapida y simple, pero mas sensible a pasos
grandes y fuerzas violentas que otros integradores mas sofisticados.

Por eso el codigo introduce:

- `max_acceleration`
- `max_velocity`
- `xsph_max_velocity`
- `velocity_damping`

Estos terminos son herramientas de ingenieria numerica.

### 11.6. `refresh_stats`

Calcula:

- numero de particulas
- velocidad maxima
- densidad maxima relativa
- hilos disponibles

Estas estadisticas sirven para observabilidad del solver. Desde la perspectiva
de CS, exponer este tipo de telemetria es muy util para entender rendimiento y
estabilidad.

## 12. Relacion exacta entre teoria y codigo

### 12.1. Continuidad -> `compute_densities`

La ecuacion de continuidad del continuo se refleja aqui como reconstruccion
local de densidad a partir del vecindario.

### 12.2. Ecuacion de estado -> `compute_pressures`

La compresibilidad debil de `WCSPH` aparece aqui.

### 12.3. Termino de presion -> `compute_accelerations`

El gradiente de presion se aproxima con el gradiente del kernel `Spiky`.

### 12.4. Viscosidad -> `compute_accelerations`

La difusion de velocidad se modela con el laplaciano del kernel.

### 12.5. Tension superficial -> `compute_accelerations`

El normal superficial y el laplaciano del campo de color sirven para detectar
superficie libre y aplicar cohesion local.

### 12.6. Fuerzas de contorno -> `boundary_acceleration` y `resolve_boundaries`

No hay boundary particles fisicos completos. En vez de eso, el contenedor se
modela con:

- fuerza de rechazo cerca del borde
- damping de choque
- correccion de posicion

Esto es una simplificacion practica y funciona bien para demos interactivas.

## 13. Estabilidad numerica: por que un SPH puede explotar

Un solver SPH explicito puede volverse inestable por varias razones:

- paso temporal demasiado grande
- presiones muy rigidas
- vecinos demasiado comprimidos
- poca viscosidad relativa
- fuerzas externas demasiado violentas
- choques bruscos con contornos
- errores acumulados de integracion

La captura de una simulacion "explosionada" suele mostrar:

- densidades enormes
- velocidades descontroladas
- dispersions violentas
- particulas saliendo despedidas

En este proyecto se han introducido varios mecanismos defensivos:

- `max_density_ratio`
  evita que la presion crezca sin limite.
- `interaction_max_acceleration`
  evita que el raton inyecte demasiada energia de golpe.
- `max_acceleration`
  evita pasos numericos imposibles.
- `max_velocity`
  evita que una particula cruce demasiado espacio por paso.
- `velocity_damping`
  extrae energia poco a poco.
- saneado de valores no finitos
  evita propagar `NaN` o `inf`.

Esto no convierte el solver en "fisica perfecta". Lo convierte en un sistema
interactivo mucho mas robusto.

## 14. Que papel tiene XSPH

El proyecto usa una correccion `XSPH`, que es una tecnica muy comun en SPH para
suavizar velocidad usando el entorno local.

Intuicion:

- si dos particulas vecinas tienen velocidades muy distintas
- se introduce una mezcla suave entre ambas
- eso reduce ruido de alta frecuencia

Ventajas:

- mejora coherencia visual
- reduce jitter
- puede ayudar a estabilidad

Riesgo:

- demasiado `XSPH` puede sobreamortiguar el fluido

## 15. Que tan "real" es esta simulacion

La respuesta correcta es: es fisicamente inspirada y numericamente fundada, pero
no es una simulacion de laboratorio ni un solver industrial de CFD.

Lo que si hace:

- discretiza ecuaciones fisicas reconocibles
- usa kernels SPH clasicos
- usa ecuacion de estado de Tait
- modela presion, viscosidad, gravedad y tension superficial
- mantiene coherencia razonable con la intuicion fisica del liquido

Lo que no hace:

- imponer incomprensibilidad estricta
- resolver contacto complejo con solidos arbitrarios
- usar validacion contra benchmarks canonicos
- modelar turbulencia realista en regimen avanzado
- usar un integrador implicito o corrector de presion sofisticado

La mejor forma de describirlo es:

```text
simulador interactivo de liquido 2D basado en WCSPH,
optimizado para claridad, estabilidad razonable y tiempo real
```

## 16. Que decisiones de ciencias de la computacion son importantes aqui

### 16.1. Estructuras de datos

La eleccion `SoA` frente a `AoS` es clave para cache y paralelismo.

### 16.2. Complejidad algoritmica

La grilla evita `O(n^2)` y lleva el solver a un coste local por vecindario.

### 16.3. Paralelismo de datos

`rayon` explota que muchas operaciones son independientes por particula.

### 16.4. Control temporal

El paso fijo y el limite de subpasos son decisiones de simulacion y de sistemas:

- mejoran estabilidad
- limitan espirales de tiempo de frame

### 16.5. Optimizacion guiada por arquitectura

Se han tomado medidas practicas:

- grilla compacta
- buffers contiguos
- precomputo de terminos como `inv_densities`
- ajuste del tamano minimo de chunks paralelos
- limitar resolucion geometrica del render de particulas

## 17. Limitaciones actuales del codigo

El proyecto actual tiene varias limitaciones claras:

1. Es 2D, no 3D.
2. Usa `WCSPH`, no un esquema casi incompresible como `DFSPH` o `IISPH`.
3. El contenedor no usa boundary particles completas.
4. El render de `nannou` sigue siendo relativamente caro para miles de
   particulas.
5. No hay perfilado automatizado ni benchmarks de regresion.
6. La fuerza del raton es una fuerza artistica, no una herramienta fisica real.
7. Los parametros estan ajustados manualmente para estabilidad visual.

## 18. Posibles mejoras futuras

Si quisieras convertir este proyecto en un motor SPH mas serio, las mejoras con
mas impacto serian:

### 18.1. Mejoras de fisica

- `DFSPH` o `IISPH` para imponer mejor incomprensibilidad
- boundary particles reales
- tension superficial mas robusta
- adaptative time stepping con criterio CFL
- validacion contra columnas de agua o dam-break benchmarks

### 18.2. Mejoras de rendimiento

- render instanciado en GPU
- compute shader para densidad y fuerzas
- ordenacion espacial aun mas agresiva
- profiling sistematico por frame

### 18.3. Mejoras de software

- tests de invariantes fisicos
- configuracion por archivo
- perfiles "fast / balanced / quality"
- separacion entre core numerico y capa grafica

## 19. Conclusion

Este proyecto es un ejemplo muy didactico de como un metodo numerico de fluidos
puede aterrizarse en codigo relativamente compacto.

Desde la fisica:

- representa masa, densidad, presion y viscosidad
- aproxima ecuaciones del continuo con kernels
- produce un liquido creible a nivel visual

Desde ciencias de la computacion:

- usa datos contiguos
- reduce complejidad con una grilla uniforme
- paraleliza trabajo por particula
- controla coste temporal y estabilidad

Desde ingenieria de simulacion:

- acepta que una demo interactiva necesita compromisos
- mezcla teoria fisica con defensas numericas
- prioriza estabilidad y rendimiento sobre pureza academica

En resumen, tu codigo no es "solo un efecto visual". Es una implementacion real
de ideas centrales de SPH y `WCSPH`, empaquetadas como un simulador interactivo
2D con decisiones concretas de arquitectura, optimizacion y estabilidad.
