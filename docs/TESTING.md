# Testing

Pruebas unitarias incluidas:

- persistencia de identidad;
- formato de ID público;
- hash y verificación Argon2id;
- round-trip del framing;
- rechazo de prefijo sobredimensionado antes de reservar memoria.

El 18 de agosto de 2026 se ejecutaron correctamente `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (4 pruebas aprobadas) y `cargo build --release` con Visual Studio Build Tools 2022. No se afirma haber realizado pruebas entre dos equipos ni por Internet, porque todavía no existe el canal de escritorio remoto.
