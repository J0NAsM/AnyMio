# AnyMio

JRemote es un proyecto de escritorio remoto personal para Windows. Usa un relay propio para que host y cliente salgan hacia la infraestructura del propietario; el ID de nueve dígitos únicamente localiza al dispositivo y nunca concede acceso.

## Estado honesto

Esta revisión contiene un núcleo de señalización ejecutable y probado: identidad local persistente, ID humano aleatorio, hashing Argon2id para una futura contraseña de acceso desatendido, framing de protocolo limitado y relay TCP de presencia/solicitudes. Aún no contiene GUI, captura, streaming, cifrado de transporte extremo a extremo ni inyección de mouse/teclado. Por ello **no es un producto de escritorio remoto V1**.

Se incluye [dist/JRemote.exe](dist/JRemote.exe) como binario portable de previsualización para probar la identidad local y el relay. No puede iniciar ni visualizar una sesión de escritorio remoto.

## Compilación

Se necesita Rust estable y Microsoft C++ Build Tools (MSVC) en Windows x64.

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

## Relay

```powershell
.\dist\JRemote.exe --relay --port 4433
```

El relay mantiene presencia y enruta solicitudes de acceso con una expiración de 60 segundos. No almacena una base de datos ni puede capturar pantallas o ejecutar entrada.

## Actualizaciones

Al iniciar, JRemote puede avisar de una versión nueva y mostrar el enlace de
descarga. Consulta [la guía de actualizaciones](docs/ACTUALIZACIONES.md) para
publicar el manifiesto y configurar su URL.

Las versiones publicadas incluyen `JRemote.exe` y `JRemoteUpdater.exe`. Cuando
existe un manifiesto más nuevo con SHA-256 válido, la ventana local ofrece el
botón **Descargar e instalar**; en consola también puede usarse
`JRemote.exe --install-update`. El ayudante descarga, verifica y reemplaza el
binario una vez que la aplicación se cierra.

## Diagnóstico local

`JRemote.exe --diagnostics` revisa de forma no intrusiva el manifiesto y la URL
del relay. `JRemote.exe --show-events 20` muestra los últimos eventos locales;
el registro no envía telemetría y la salida se limita a 200 registros.

## Automatización

GitHub Actions ejecuta formato, lints, pruebas y una compilación de Windows en
cada cambio a `main`. Al publicar una etiqueta `vX.Y.Z` que coincida con la
versión de `Cargo.toml`, el workflow crea el Release, genera el instalador y
actualiza `update.json`. Consulta [la guía de compilación](docs/BUILD.md) para
la ejecución local.

## Seguridad actual

- El ID público es un localizador aleatorio, no una contraseña.
- Las futuras contraseñas desatendidas se almacenarán mediante Argon2id con salt aleatorio.
- Cada mensaje está versionado, serializado explícitamente y limitado a 64 KiB antes de reservar memoria.
- El relay valida registro, propiedad de solicitudes y vencimiento antes de reenviar una aceptación o rechazo.

Consulta [docs/ARQUITECTURA.md](docs/ARQUITECTURA.md), [docs/PROTOCOLO.md](docs/PROTOCOLO.md) y [docs/SEGURIDAD.md](docs/SEGURIDAD.md).
Las dependencias y sus licencias se registran en [docs/DEPENDENCIAS.md](docs/DEPENDENCIAS.md).
