# JRemote / AnyMio — contexto

Fecha de revisión estructural: 2026-09-14. Identificador: anymio.

## Producto y alcance
Prototipo de escritorio remoto Windows; captura/control reales pendientes según README.

Tecnología y persistencia: Rust, eframe, Tokio; relay propio; sin DB identificada

Dependencias y servidor: Relay; distribución Windows y compilador Rust declarado en Cargo.toml.

## Lectura obligatoria
1. [Contexto general de Vaults](<../../Vaults/jmartinez/Ecosistema/contexto.md>) y [reglas generales](<../../Vaults/jmartinez/Ecosistema/reglas.md>).
2. [Reglas particulares](reglas.md) y [ejecución y validación](ejecucion.md).
3. Documentación y decisiones del componente que vaya a cambiar.

## Límites
No afirmar control remoto completo. Mantener visibilidad y consentimiento del operador.

## Fuentes técnicas
- [Cargo.toml](<../Cargo.toml>)

## Documentación conservada
- [README.md](<../README.md>)
- [docs/ACTUALIZACIONES.md](<../docs/ACTUALIZACIONES.md>)
- [docs/ARQUITECTURA.md](<../docs/ARQUITECTURA.md>)
- [docs/BUILD.md](<../docs/BUILD.md>)
- [docs/DEPENDENCIAS.md](<../docs/DEPENDENCIAS.md>)
- [docs/LIMITACIONES.md](<../docs/LIMITACIONES.md>)
- [docs/PROTOCOLO.md](<../docs/PROTOCOLO.md>)
- [docs/ROADMAP.md](<../docs/ROADMAP.md>)
- [docs/SEGURIDAD.md](<../docs/SEGURIDAD.md>)
- [docs/TESTING.md](<../docs/TESTING.md>)

## Estado verificable
La metadata está en [proyecto.json](proyecto.json). STATUS y último despliegue permanecen NO DETERMINADO hasta contar con evidencia. Una revisión documental no valida el funcionamiento de la aplicación.
[Seguimiento de correcciones y dependencias externas](<../../Vaults/jmartinez/Ecosistema/seguimiento.md>).

No copiar versiones, estado de Git o resultados históricos como si fueran hechos permanentes. Al cambiar una fuente técnica, revisar el contexto y actualizar su hash solo después de comprobar coherencia.

<!-- BEGIN ECOSYSTEM DETAILS -->
## Documentos por tarea

- [arquitectura.md](<arquitectura.md>)
- [testing.md](<testing.md>)
- [despliegue.md](<despliegue.md>)
<!-- END ECOSYSTEM DETAILS -->
