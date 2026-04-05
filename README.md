# fluid-simulation

Simulador de fluidos en Rust con `nannou` para visualizacion y un solver
`WCSPH` multihilo para la dinamica.

## Que incluye

- Densidad por sumatorio SPH con kernel `Poly6`
- Presion con ecuacion de estado de Tait
- Fuerza de presion simetrica con gradiente `Spiky`
- Viscosidad laminar con Laplaciano SPH
- Tension superficial basada en campo de color
- Suavizado `XSPH`
- Grilla uniforme para busqueda local de vecinos
- Pasos paralelos con `rayon` en densidad, fuerzas e integracion
- Paso de tiempo fijo para mejorar estabilidad

## Controles

- Click izquierdo: atrae el fluido
- Click derecho: repele el fluido

## Ejecutar

```bash
cargo run --release
```

## Validacion

```bash
cargo test
cargo check --release
```

## Documentacion tecnica

- `docs/ARTICULO_SPH.md`: articulo tecnico sobre SPH, fisica, arquitectura y
  relacion entre teoria y codigo

## Nota

Es un solver 2D interactivo fisicamente fundamentado, no una implementacion CFD
completa de produccion. Los coeficientes estan ajustados para estabilidad visual y
rendimiento en tiempo real dentro del contenedor de la demo.
