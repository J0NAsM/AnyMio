# Ejecución y validación

## Requisitos y límites
Relay; distribución Windows y compilador Rust declarado en Cargo.toml.

No afirmar control remoto completo. Mantener visibilidad y consentimiento del operador.

## Comandos declarados
Ejecutar desde el directorio indicado, después de revisar sus efectos. Esta tabla acredita que existe el script, no que haya pasado recientemente.
Los comandos de prueba pueden escribir archivos o datos. Builds móviles requieren SDK/firma y Maven puede ejecutar pruebas de integración.

| Fuente | Directorio relativo a la raíz | Script o propósito | Comando |
|---|---|---|---|
| Cargo.toml | . | Pruebas | cargo test |
| Cargo.toml | . | Build | cargo build --release |
| Cargo.toml | . | Formato | cargo fmt --check |

## Configuración y despliegue encontrados
- [.github/workflows/ci.yml](<../.github/workflows/ci.yml>)
- [.github/workflows/release.yml](<../.github/workflows/release.yml>)

Despliegue efectivo: NO DETERMINADO. No ejecutar Compose, migraciones o arranque contra datos compartidos por inferencia.

## Puertos
Consultar [registro global](<../../Vaults/jmartinez/Infraestructura/Puertos/registro.json>) antes de iniciar varias aplicaciones. Se conservan los puertos actuales; si un proceso ajeno ocupa uno, informar y no detenerlo.

## Cierre de un cambio
Registrar comando, entorno, revisión, fecha y resultado real en proyecto.json o en el informe de validación del cambio. Actualizar documentación afectada y revisar el diff. No confundir la existencia de CI con un resultado aprobado.
