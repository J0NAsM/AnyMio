# Hoja de ruta de mejoras

Cada punto tiene un estado comprobable. Las funciones de control remoto real se
mantienen separadas de la señalización hasta disponer de cifrado E2E, consentimiento
visible y pruebas entre equipos.

| # | Mejora | Entrega verificable |
|---:|---|---|
| 1 | Actualización desde GitHub | Manifiesto y Release verificados por SHA-256 |
| 2 | Interfaz local | Ventana de estado, identidad y acciones visibles |
| 3 | Buscar actualizaciones | Acción manual y resultado visible |
| 4 | Progreso de descarga | Estado del actualizador y errores recuperables |
| 5 | Historial de cambios | Notas del manifiesto y registro local |
| 6 | Actualización configurable | Canales estable/beta y descarga bajo acción explícita |
| 7 | Recuperación | Copia del binario anterior y rollback documentado |
| 8 | Errores comprensibles | Diagnóstico y registro estructurado local |
| 9 | Configuración persistente | Archivo atómico versionado en datos locales |
| 10 | Idioma | Preferencia español/inglés preparada |
| 11 | Instalador | Script NSIS y artefacto automatizado |
| 12 | Firma de código | Flujo preparado; requiere certificado externo |
| 13 | Firma de manifiesto | Flujo preparado; requiere clave privada externa |
| 14 | Canales | URL de manifiesto por canal |
| 15 | Informe local | Registro de eventos sin telemetría oculta |
| 16 | Auditoría | Eventos de actualización y futuras sesiones |
| 17 | Diagnóstico de red | Comprobación de URL de relay y actualización |
| 18 | Prueba de relay | HTTPS por `HEAD` o puerto WSS por TCP, sin autenticación |
| 19 | Reconexión | Backoff limitado de cinco intentos y nueva acción explícita |
| 20 | Relay configurable | URL persistente validada |
| 21 | Estado del relay | Estado local visible, sin fingir conectividad |
| 22 | Dispositivos conocidos | Lista local con claves/IDs y etiquetas |
| 23 | Nombres de equipo | Alias locales editables |
| 24 | Consentimiento | Modelo y registro; sin acceso oculto |
| 25 | Historial de sesiones | Modelo persistente y con límite de retención |
| 26 | Protección ante errores | Límites y bloqueos temporales preparados |
| 27 | Credenciales | Argon2id existente y campos sin secretos en logs |
| 28 | Copia de configuración | Exportación/importación explícita preparada |
| 29 | Pruebas | Unitarias, CI y pruebas de actualización |
| 30 | Panel de releases | GitHub Actions publica artefactos y manifiesto |

## Requisitos externos

GitHub debe exponer `update.json` y los assets públicamente (o deben alojarse en
un HTTPS público). Authenticode requiere un certificado de firma y la firma del
manifiesto requiere una clave privada protegida. La captura de escritorio, vídeo,
entrada remota y acceso desatendido no se declararán implementados hasta contar
con un diseño E2E, consentimiento visible y pruebas reales de seguridad.
