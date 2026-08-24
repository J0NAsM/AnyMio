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
# Seguridad de actualizaciones

El canal de actualización acepta únicamente URLs HTTPS. Las versiones nuevas
deben declarar un SHA-256 de 64 caracteres; el proceso auxiliar vuelve a
calcularlo mientras descarga el ejecutable y rechaza cualquier diferencia. No
reemplaza archivos arbitrarios: solamente `JRemote.exe` situado junto al
actualizador.

El repositorio público y la cuenta con permiso de publicar releases forman parte
de la cadena de confianza. El hash evita corrupción o un enlace equivocado, pero
no reemplaza una firma de código. La siguiente mejora de seguridad recomendada
es firmar `JRemote.exe`, `JRemoteUpdater.exe` y el instalador con Authenticode.

El workflow de Release admite esa firma sin exponer el certificado: configura
los secretos `WINDOWS_CERTIFICATE_BASE64` y `WINDOWS_CERTIFICATE_PASSWORD` en
GitHub. Si no están presentes, publica artefactos sin firma y lo declara en el
log del workflow.
