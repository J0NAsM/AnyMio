# Seguridad

## Implementado

- UUID v4 e ID de dispositivo obtenidos con CSPRNG del sistema.
- Argon2id (19 MiB, 3 iteraciones, un hilo y salt aleatorio) para hashes de contraseñas desatendidas.
- Validación de versión, estado de registro, tamaño de paquetes, destinatario de aceptación y expiración.
- Relay sin persistencia de credenciales ni telemetría, con límite de solicitudes y bloqueo temporal tras cinco fallos de autenticación.
- Registro local limitado de actividad y del ciclo de consentimiento, sin telemetría oculta.
- Actualizaciones HTTPS con SHA-256 obligatorio y firma Ed25519 opcional del manifiesto cuando la versión se compila con una clave pública.

## Pendiente, por tanto no garantizado todavía

- TLS 1.3 / cifrado E2E de datos de sesión.
- Autorización remota de una sesión E2E real.
- Nonces, anti-replay y claves de sesión.
- Protección del archivo de identidad con DPAPI.
- Controles de DoS de infraestructura y monitoreo de producción.

La falta de estas capas significa que el código actual sólo debe usarse en desarrollo local, nunca como relay expuesto a Internet.

## Firma de manifiesto

Genera y guarda la clave privada Ed25519 fuera del repositorio. Publica la clave
pública (64 caracteres hexadecimales) como variable de GitHub
`UPDATE_MANIFEST_PUBLIC_KEY`, y el valor privado como secreto
`UPDATE_MANIFEST_SIGNING_PRIVATE_KEY`. El workflow compila AnyMio con la clave
pública, firma `update.json` mediante `JRemoteManifestSigner.exe` y elimina el
archivo temporal de clave. Si no se configuran ambas, la publicación conserva la
protección SHA-256 pero no declara una firma de manifiesto.
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

El relay también limita solicitudes pendientes y bloquea temporalmente una IP
tras cinco fallos de autenticación dentro de un minuto. Es una medida de
contención, no un sustituto de TLS, limitación de red perimetral ni monitoreo de
infraestructura.
