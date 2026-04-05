# Presentacion universitaria: SPH, implementacion en Rust y lectura desde ingenieria quimica

## Enfoque general

Esta presentacion deberia responder tres preguntas:

1. Que es SPH y por que sirve para simular fluidos.
2. Como se traduce esa teoria a un solver numerico programado en Rust.
3. Que fenomenos de interes en ingenieria quimica pueden simularse con este tipo de metodo y cuales son sus limites.

La idea no es solo mostrar una demo grafica, sino conectar el metodo con conceptos de fenomenos de transporte, CFD, balances locales, estabilidad numerica y modelado de operaciones con superficie libre.

## Nivel academico recomendado

El tono deberia ser de grado avanzado o inicio de master en ingenieria quimica / ingenieria de procesos:

- Explicar la base fisica desde continuidad y cantidad de movimiento.
- Presentar la discretizacion SPH con ecuaciones, no solo con intuicion visual.
- Justificar decisiones numericas y de estructuras de datos.
- Discutir que parte del modelo es fisica, que parte es aproximacion numerica y que parte es estabilizacion practica.
- Cerrar con aplicaciones reales en ingenieria quimica y criterios de validez.

---

# Guion propuesto de diapositivas

## Diapositiva 1. Titulo y objetivo

**Titulo sugerido:**
`Smoothed Particle Hydrodynamics (SPH): fundamento, implementacion en Rust y aplicaciones en ingenieria quimica`

**Contenido:**

- SPH como metodo lagrangiano de simulacion de fluidos por particulas.
- Ejemplo practico: solver `WCSPH` 2D interactivo programado en Rust.
- Objetivo: entender el metodo, su implementacion y su utilidad en problemas de procesos con superficie libre.

**Mensaje oral clave:**
La presentacion no trata solo de "animar agua", sino de mostrar como un modelo continuo de fluidos puede convertirse en un algoritmo de particulas y que lectura tiene eso en ingenieria quimica.

---

## Diapositiva 2. Motivacion desde ingenieria quimica

**Pregunta inicial: por que interesaria SPH en ingenieria quimica?**

**Ejemplos de fenomenos relevantes:**

- Movimiento con superficie libre en tanques, recipientes y canales.
- Llenado, vaciado, sloshing y oleaje interno en equipos de proceso.
- Chorros, salpicadura y ruptura de masas liquidas.
- Mezcla cualitativa en geometria abierta o altamente deformable.
- Transporte de pulpas o suspensiones si el modelo se extiende.
- Interaccion fluido-pared en recipientes simples.

**Contraste con CFD euleriana tradicional:**

- En una malla fija, la interfaz libre puede exigir tratamiento adicional.
- En SPH, la interfaz aparece de forma natural porque el fluido se representa con particulas materiales.

**Mensaje oral clave:**
SPH es especialmente atractivo cuando la geometria de la interfase cambia mucho, por ejemplo en salpicaduras o llenado transitorio, donde una formulacion estrictamente mallada puede requerir reconstruccion o seguimiento de interfaz.

---

## Diapositiva 3. Que es SPH

**Definicion:**
`SPH` es un metodo numerico lagrangiano y sin malla volumetrica interna donde el fluido se discretiza como particulas que transportan masa, posicion, velocidad, densidad y presion.

**Idea central:**

- Cada particula representa una porcion finita de fluido.
- Las propiedades locales se reconstruyen por sumas ponderadas sobre particulas vecinas.
- Esa ponderacion se hace mediante un kernel de suavizado con radio de soporte `h`.

**Comparacion conceptual:**

- Euleriano: observo que ocurre en cada celda fija del espacio.
- Lagrangiano/SPH: sigo cada elemento material de fluido en movimiento.

**En el codigo:**

- Las particulas no se guardan como un `Vec<Particle>`, sino como arreglos separados:
  `positions`, `velocities`, `densities`, `pressures`, `accelerations`, etc.
- Esto aparece en `src/sph.rs` dentro de `SphSimulation`.

---

## Diapositiva 4. Base fisica: de Navier-Stokes al modelo discreto

**Ecuaciones de referencia en forma material:**

```text
Continuidad:
d rho / dt + rho * div(v) = 0

Cantidad de movimiento:
rho * dv/dt = -grad(p) + mu * nabla^2(v) + rho*g + f_superficie + f_frontera
```

**Lectura desde ingenieria quimica:**

- `rho`: densidad local del fluido.
- `p`: presion mecanica.
- `mu`: efecto viscoso o difusion de cantidad de movimiento.
- `g`: cuerpo externo, por ejemplo gravedad.
- Terminos extra pueden representar tension superficial o interaccion con paredes.

**Paso conceptual SPH:**

En vez de aproximar derivadas sobre una malla, SPH reemplaza campos y operadores por sumatorios sobre vecinos:

```text
A(x_i) ~= sum_j m_j * A_j / rho_j * W(|x_i - x_j|, h)
```

**Mensaje oral clave:**
SPH no elimina la fisica; cambia la forma de discretizarla. La calidad del resultado depende tanto del modelo fisico como del kernel, del radio de soporte y de la integracion temporal.

---

## Diapositiva 5. Kernels SPH y estimacion de densidad

**Formula de densidad:**

```text
rho_i = sum_j m_j * W(|x_i - x_j|, h)
```

**Interpretacion:**

- Particulas cercanas aportan mas a la densidad local.
- Fuera de `h`, la contribucion es cero.
- La densidad no se impone por celda, se reconstruye localmente desde el vecindario.

**En el codigo Rust:**

```rust
fn compute_densities(&mut self) {
    self.densities.par_iter_mut().enumerate().for_each(|(index, density)| {
        let position_i = positions[index];
        let mut value = 0.0;

        grid.for_each_neighbor(particle_cells[index], |neighbor| {
            let delta = position_i - positions[neighbor];
            value += particle_mass * kernels.poly6(delta.length_squared());
        });

        *density = value.max(minimum_density);
    });
}
```

**Que destacar en clase:**

- Se usa `Poly6` para densidad.
- Se evalua solo sobre vecinos locales.
- Hay un limite inferior `minimum_density` para evitar divisiones inestables o densidades no fisicas por ruido numerico.

---

## Diapositiva 6. Presion en WCSPH: ecuacion de estado de Tait

**Modelo usado:**
El solver es `WCSPH` (`Weakly Compressible SPH`), es decir, el fluido se trata como debilmente compresible.

**Ecuacion de estado:**

```text
p_i = k * ((rho_i / rho_0)^gamma - 1)
```

**Interpretacion fisica:**

- Si `rho_i` supera la densidad de reposo `rho_0`, aparece presion positiva que tiende a expandir localmente el fluido.
- `k` controla la rigidez del fluido frente a compresion.
- `gamma` controla la no linealidad.

**En el codigo:**

```rust
let ratio = (clamped_density / rest_density).clamp(1.0, max_density_ratio);
let value = pressure_stiffness * (ratio.powf(gamma) - 1.0);

*pressure = value;
*inv_density = density_inv;
*pressure_term = value * density_inv * density_inv;
```

**Lectura desde ingenieria quimica:**

Este tratamiento es razonable para simulacion interactiva de liquidos casi incomprensibles, pero no equivale a un solver industrial de flujo incompresible estricto. Si se necesitara prediccion cuantitativa fina de campo de presion, habria que validar y posiblemente pasar a esquemas como IISPH/DFSPH o a CFD euleriana mas clasica.

---

## Diapositiva 7. Fuerzas: presion, viscosidad, tension superficial, gravedad y paredes

**Terminos principales en el solver:**

```text
a_i = a_presion + a_viscosidad + g + a_tension_superficial + a_frontera + a_interaccion
```

**Presion:**
Se usa una forma simetrica basada en el gradiente del kernel `Spiky`, lo que ayuda a reducir sesgos entre pares de particulas.

**Viscosidad:**
Se modela como suavizado de diferencias de velocidad entre vecinos usando el laplaciano del kernel. Desde ingenieria quimica, este termino representa difusion de cantidad de movimiento.

**Tension superficial:**
Se estima un normal de superficie y un laplaciano de campo de color. Solo se aplica si la magnitud del normal supera un umbral, lo que indica que la particula esta cerca de una interfaz libre.

**Fronteras:**
No hay una pared mallada compleja; se usa repulsion cerca del borde, amortiguamiento y correccion de posicion tras integrar.

**Interaccion con raton:**
Es una fuerza numerica externa util para manipular la demo, pero no representa directamente un actuador fisico real.

**En el codigo:**
Todo esto se concentra en `compute_accelerations`, `boundary_acceleration`, `resolve_boundaries` e `interaction_acceleration`.

---

## Diapositiva 8. Pipeline numerico del solver

**Secuencia por paso temporal en `SphSimulation::step`:**

```rust
pub fn step(&mut self, dt: f32, interaction: Option<Interaction>) {
    self.grid.rebuild(&self.positions);
    self.compute_densities();
    self.compute_pressures();
    self.compute_accelerations(interaction);
    self.integrate(dt);
}
```

**Interpretacion numerica:**

1. Reconstruir vecindad local.
2. Estimar densidad por sumatorio SPH.
3. Convertir densidad en presion por ecuacion de estado.
4. Calcular aceleraciones de todas las particulas.
5. Integrar velocidad y posicion.

**Comentario de estabilidad:**
Este es un esquema explicito. Es simple y rapido, pero sensible a pasos `dt` demasiado grandes, presiones muy rigidas o fuerzas externas abruptas.

---

## Diapositiva 9. Integracion temporal y control de estabilidad

**En `src/main.rs`:**

```rust
const FIXED_TIME_STEP: f32 = 1.0 / 240.0;
const MAX_STEPS_PER_FRAME: usize = 6;
```

**En el bucle de simulacion:**

```rust
while model.accumulator >= FIXED_TIME_STEP && steps < MAX_STEPS_PER_FRAME {
    model.simulation.step(FIXED_TIME_STEP, interaction);
    model.accumulator -= FIXED_TIME_STEP;
    steps += 1;
}
```

**Por que esto importa:**

- Un paso fijo reduce dependencia de la fisica respecto al framerate.
- Limitar subpasos por frame evita una espiral de lag si el render cae.
- El solver ademas limita densidad, aceleracion, velocidad, correccion XSPH y sanea valores no finitos.

**En `src/sph.rs`:**

- `max_density_ratio`
- `max_acceleration`
- `max_velocity`
- `xsph_max_velocity`
- `velocity_damping`
- `clamp_magnitude`

**Lectura academica:**
Estos limites son estabilizacion numerica e ingenieria de simulacion, no leyes fisicas fundamentales. En una presentacion universitaria conviene decirlo explicitamente.

---

## Diapositiva 10. Busqueda de vecinos y coste computacional

**Problema naive:**
Si cada particula compara contra todas, el coste es `O(n^2)`.

**Solucion en el proyecto: `UniformGrid`**

- El dominio se divide en celdas.
- Cada particula se asigna a una celda.
- Para cada particula solo se revisa su celda y celdas adyacentes.
- El coste efectivo se acerca a `O(n * k)`, con `k` numero medio de vecinos locales.

**Detalle de estructura de datos:**

- `cell_counts`
- `cell_offsets`
- `sorted_particles`
- `particle_cells`
- `neighbor_cells`

**Por que esto es relevante en ingenieria:**
En simulacion de procesos, el problema no es solo "tener una ecuacion correcta", sino poder evaluarla a coste razonable para miles o millones de elementos. La estructura de vecinos es una decision algoritmica clave.

---

## Diapositiva 11. Paralelizacion con Rayon y organizacion de memoria

**Decision de arquitectura:**
El solver usa una organizacion `Structure of Arrays`:

- `positions: Vec<Vec2>`
- `velocities: Vec<Vec2>`
- `densities: Vec<f32>`
- `pressures: Vec<f32>`
- `accelerations: Vec<Vec2>`

**Ventajas:**

- Mejor localidad de cache para bucles por campo.
- Mas facil paralelizar por particulas.
- Menos carga de datos no necesarios en cada etapa.

**Paralelizacion:**
Se usa `rayon` con `par_iter_mut()` en densidad, presion, aceleraciones, integracion y estadisticas.

**Ejemplo conceptual:**

```rust
self.positions
    .par_iter_mut()
    .zip(self.velocities.par_iter_mut())
    .zip(self.accelerations.par_iter().copied())
    .for_each(|(((position, velocity), acceleration), xsph)| {
        *velocity += acceleration * dt;
        *position += (*velocity + xsph) * dt;
    });
```

**Mensaje oral clave:**
Este solver es un buen ejemplo de como una formulacion fisica lagrangiana puede mapearse naturalmente a paralelismo de datos, siempre que cada fase este organizada para lecturas compartidas y escrituras independientes.

---

## Diapositiva 12. Visualizacion e interpretacion de resultados

**En `src/main.rs`:**

- Se renderiza cada particula como una elipse.
- El color se hace variar con la velocidad y la densidad relativa.
- Se muestra un HUD con numero de particulas, hilos, FPS, velocidad maxima, densidad relativa maxima y subpasos por frame.

**Interpretacion importante:**
El color no es una variable termodinamica del modelo; es una codificacion visual derivada de velocidad y densidad. No debe interpretarse como temperatura o composicion a menos que el solver incluya explicitamente esos balances escalares.

**Puente hacia ingenieria quimica:**
Si se quisiera visualizar concentracion de soluto, fraccion masica, temperatura o edad de mezcla, habria que anadir ecuaciones de transporte adicionales por particula, no solo cambiar el mapa de color.

---

## Diapositiva 13. Que puede simularse en ingenieria quimica con este tipo de SPH

**Casos donde SPH es especialmente util:**

- Sloshing en tanques y recipientes parcialmente llenos.
- Transitorios de llenado y vaciado.
- Derrames, oleaje local y salpicadura.
- Chorros y ruptura de volumenes liquidos con superficie libre.
- Mezcla cualitativa en dominios donde la interfaz se deforma mucho.
- Flujos con frontera movil si el modelo se extiende.
- Fenomenos capilares cualitativos si la tension superficial esta calibrada.

**Extensiones posibles de interes quimico:**

- Transporte de especie:
  anadir una variable `c_i` y una ecuacion de adveccion-difusion SPH.
- Transferencia de calor:
  anadir `T_i`, conduccion termica y terminos fuente.
- Fluidos no newtonianos:
  hacer `mu` dependiente de tasa de deformacion, concentracion o temperatura.
- Suspension solido-liquido:
  usar particulas de fases distintas y leyes de acoplamiento/interfase.
- Multifase liquido-liquido:
  introducir propiedades por fase, tension interfacial y posibles modelos de miscibilidad.

**Criterio practico:**
El codigo actual es una base didactica para liquido simple 2D. Para usarlo como herramienta de ingenieria quimica habria que extender fisica, condiciones de frontera y validacion experimental.

---

## Diapositiva 14. Que NO deberia afirmarse con este solver

**Limitaciones de la implementacion actual:**

- Es 2D, no 3D.
- Usa `WCSPH`, no incomprensibilidad estricta.
- No incluye transporte de energia ni especies.
- No modela reacciones quimicas.
- No modela turbulencia industrial con cierre validado.
- No usa geometria de equipo compleja ni frontera solida detallada.
- No esta validado frente a benchmarks experimentales de proceso.
- Los parametros estan ajustados para estabilidad y visualizacion en tiempo real, no para identificar propiedades reologicas reales.

**Mensaje oral clave:**
Desde el punto de vista academico, esta demo es excelente para explicar el metodo y su implementacion, pero no deberia presentarse como una prediccion cuantitativa de un reactor, mezclador o columna real sin extension y validacion.

---

## Diapositiva 15. Como conectar este codigo con un caso de ingenieria quimica en la exposicion

**Caso didactico recomendado: tanque parcialmente lleno sometido a perturbacion**

**Lectura fisica:**

- La gravedad genera redistribucion hidrostatica.
- La presion SPH aparece cuando la densidad local se comprime.
- La viscosidad disipa gradientes de velocidad.
- La tension superficial actua sobre la superficie libre.
- Las paredes devuelven momento y disipan parte de la energia de impacto.

**Lectura de proceso:**

- Este caso se parece a sloshing en recipientes de almacenamiento o transporte.
- Si el fluido fuese mas viscoso, se esperaria amortiguamiento mas fuerte.
- Si la tension superficial fuese dominante en escala pequena, la forma local de la interfaz cambiaria.
- Si se quisiera una mezcla binaria o una solucion concentrada, faltaria incorporar balance de especie y propiedades dependientes de composicion.

**Como presentarlo:**
Mostrar primero la fenomenologia, luego senalar exactamente en que funcion del codigo entra cada efecto fisico.

---

## Diapositiva 16. Relacion directa teoria-codigo

| Concepto teorico | Donde aparece en el codigo | Comentario |
|---|---|---|
| Radio de soporte `h` | `SphConfig::smoothing_radius`, `KernelSet` | Define vecindario e influencia local |
| Densidad SPH | `compute_densities` | Sumatorio con kernel `Poly6` |
| Ecuacion de estado | `compute_pressures` | Presion WCSPH tipo Tait |
| Gradiente de presion | `compute_accelerations` + `spiky_gradient` | Fuerza simetrica entre particulas |
| Viscosidad | `compute_accelerations` + `viscosity_laplacian` | Difusion de cantidad de movimiento |
| Tension superficial | `compute_accelerations` | Normal superficial + laplaciano de color |
| Interaccion externa | `interaction_acceleration` y `active_interaction` | Atraer/repeler con raton |
| Condicion de pared | `boundary_acceleration`, `resolve_boundaries` | Rebote, amortiguamiento y clamp |
| Integracion temporal | `integrate` y bucle de `update` | Esquema explicito con `dt` fijo |
| Paralelismo | `par_iter_mut`, `rayon::current_num_threads()` | Data parallel por particulas |
| Render/HUD | `view` en `src/main.rs` | Visualizacion e instrumentacion |

---

## Diapositiva 17. Discusion critica: cuando elegir SPH y cuando no

**SPH conviene si:**

- La superficie libre y grandes deformaciones son centrales.
- El dominio material cambia de forma de manera fuerte.
- Se prioriza una representacion lagrangiana natural.
- Se busca una demo fisica didactica o prototipado rapido con particulas.

**SPH puede no ser la mejor primera opcion si:**

- Se necesita presion incomprensible muy precisa en regimen industrial.
- La geometria interna del equipo y capas limite cerca de pared dominan el problema.
- Se requiere simulacion con transferencia de calor/especies altamente calibrada y validada.
- Se necesita resolver problemas 3D grandes con garantia fuerte de convergencia y benchmark industrial.

**Mensaje oral clave:**
En ingenieria quimica, la eleccion del metodo numerico debe responder al fenomeno dominante, la escala, la geometria, los observables requeridos y el nivel de validacion exigido.

---

## Diapositiva 18. Conclusiones

**Ideas finales:**

- SPH aproxima un fluido continuo mediante particulas y kernels de suavizado.
- El solver Rust del proyecto implementa un `WCSPH` 2D con densidad `Poly6`, presion tipo Tait, gradiente `Spiky`, viscosidad, tension superficial, XSPH, grilla uniforme y paralelizacion con `rayon`.
- La arquitectura del codigo separa claramente orquestacion/render (`src/main.rs`) y fisica numerica (`src/sph.rs`).
- Para ingenieria quimica, la mayor fortaleza de este enfoque esta en fenomenos con superficie libre y movimiento material complejo.
- Para uso predictivo real en procesos, harian falta extension de fisica, geometria, validacion y posiblemente esquemas mas robustos de incomprensibilidad.

**Cierre oral sugerido:**
Este proyecto funciona muy bien como puente entre fenomenos de transporte y computacion cientifica: muestra como una ecuacion continua termina convirtiendose en estructuras de datos, kernels, busqueda de vecinos, paralelismo y decisiones explicitas de estabilidad numerica.

---

# Recomendaciones de estilo para convertir este Markdown en una exposicion

- Usar entre 12 y 18 diapositivas; si el tiempo es corto, fusionar las de paralelismo, visualizacion y discusion critica.
- Incluir 2 o 3 ecuaciones maximas por bloque teorico y explicarlas fisicamente, no solo leerlas.
- Mostrar fragmentos cortos de codigo, no pantallas enormes.
- En cada fragmento de Rust, responder tres cosas:
  que calcula, por que esa forma es SPH, y que implicacion fisica/numerica tiene.
- Si se lleva a ingenieria quimica, evitar prometer reacciones, transferencia de calor o composicion si el solver actual no las resuelve.
- Una buena secuencia pedagogica es:
  fenomeno fisico -> ecuacion -> discretizacion SPH -> funcion Rust -> interpretacion en procesos.

# Bibliografia tecnica sugerida para la ultima diapositiva

- Monaghan, J. J. Smoothed Particle Hydrodynamics. Revision clasica del metodo SPH.
- Muller et al. Particle-Based Fluid Simulation for Interactive Applications. Referencia muy usada en graficos y SPH interactivo.
- Liu & Liu. Smoothed Particle Hydrodynamics: A Meshfree Particle Method. Texto mas sistematico sobre fundamentos y variantes.
- Libros o apuntes de CFD y fenomenos de transporte usados en la asignatura para contextualizar Navier-Stokes, continuidad y difusion de momento.

# Si quieres mejorar aun mas la presentacion

Podria prepararte una segunda version de este Markdown ya con formato de "diapositiva + notas del ponente", o una version mas orientada a defensa oral de 10 minutos frente a tribunal universitario.
