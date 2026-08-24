# Testing

Pruebas unitarias incluidas:

- persistencia de identidad;
- formato de ID público;
- hash y verificación Argon2id;
- round-trip del framing;
- rechazo de prefijo sobredimensionado antes de reservar memoria.
- verificación de firmas de identidad;
- comparación de versiones de actualización;
- rechazo de manifiestos sin SHA-256, hashes inválidos y URLs sin HTTPS;
- validación de las restricciones del actualizador auxiliar.

La CI de GitHub ejecuta `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` y `cargo build --release` en Windows. No se afirma haber realizado pruebas entre dos equipos ni por Internet, porque todavía no existe el canal de escritorio remoto.
