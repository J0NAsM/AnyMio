# Arquitectura actual

## Componentes implementados

`JRemote --relay` abre un listener TCP y mantiene dos tablas en memoria:

- `device_id → sender`: presencia de endpoints registrados.
- `request_id → solicitud pendiente`: asocia un cliente con el host destino durante un máximo de 60 segundos.

El endpoint actual genera o recupera una identidad de `%LOCALAPPDATA%/JRemote/JRemote/identity.json`. Sus campos son UUID v4, ID de nueve dígitos, fecha y nombre opcional. No se deriva de MAC, hostname o IP.

## Límites explícitos

No hay todavía canal de datos de sesión. En consecuencia el relay sólo procesa señalización y no existe captura de pantalla, codec, viewer ni inyección de entrada. La futura sesión deberá ser un canal cifrado de extremo a extremo independiente de la señalización: el relay podrá asociar conexiones por un token de sesión, sin descifrar video ni eventos de entrada.

El archivo local `session-history.json` conserva hasta 200 eventos del ciclo de
consentimiento (solicitud, aprobación o rechazo). Es una pista de auditoría y
no implica que se haya iniciado una sesión de escritorio.

La futura capa de sesión usa una política de reconexión limitada: cinco intentos
con espera exponencial de 1, 2, 4, 8 y 16 segundos. Tras agotarse requiere una
nueva acción explícita del usuario; no habilita acceso desatendido.

## Próxima arquitectura necesaria para V1

1. GUI visible de host/cliente y solicitud local de consentimiento.
2. Handshake autenticado y canal E2E (TLS 1.3 o QUIC con autenticación de claves de dispositivo).
3. Captura Windows Graphics Capture, codificación Media Foundation y viewer.
4. Entrada con `SendInput`, restringida a una sesión autorizada y siempre con indicador local.

Estas fases no deben declararse listas sin pruebas entre dos equipos.
