# Despliegue — JRemote / AnyMio

Dependencias: Relay; distribución Windows y compilador Rust declarado en Cargo.toml.

No afirmar control remoto completo. Mantener visibilidad y consentimiento del operador.

## Artefactos de configuración encontrados
- [.github/workflows/ci.yml](<../.github/workflows/ci.yml>)
- [.github/workflows/release.yml](<../.github/workflows/release.yml>)

Esto no acredita despliegue efectivo. Host, dominio administrado, certificado, versión desplegada y rollback probado: NO DETERMINADO. Antes de producción comprobar build, datos persistentes, variables privadas, health checks y restauración. No cambiar identidad de volúmenes o redes sin inventariar los existentes.
