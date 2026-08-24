# Seguridad

## Implementado

- UUID v4 e ID de dispositivo obtenidos con CSPRNG del sistema.
- Argon2id (19 MiB, 3 iteraciones, un hilo y salt aleatorio) para hashes de contraseñas desatendidas.
- Validación de versión, estado de registro, tamaño de paquetes, destinatario de aceptación y expiración.
- Relay sin persistencia de credenciales ni telemetría.

## Pendiente, por tanto no garantizado todavía

- TLS 1.3 / cifrado E2E de datos de sesión.
- Autorización local visible, rate limiting y bloqueo progresivo.
- Nonces, anti-replay y claves de sesión.
- Protección del archivo de identidad con DPAPI.
- Auditoría y controles de DoS de conexiones.

La falta de estas capas significa que el código actual sólo debe usarse en desarrollo local, nunca como relay expuesto a Internet.
