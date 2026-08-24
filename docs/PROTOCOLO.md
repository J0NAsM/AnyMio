# JREMOTE/1 — señalización

El transporte actual es TCP delimitado por longitud: `u32 big-endian` seguido de JSON UTF-8. Un prefijo mayor que 65.536 bytes se rechaza antes de asignar el búfer. Todos los mensajes usan una etiqueta `type` explícita.

Flujo:

1. El peer envía `HELLO { protocol_version: 1 }`.
2. Registra `REGISTER { device_id, identity }`.
3. El cliente consulta `LOOKUP` o envía `CONNECT_REQUEST` con UUID aleatorio y destino.
4. El relay reenvía la solicitud al host registrado.
5. El host emite `CONNECT_ACCEPT` o `CONNECT_REJECT`; el relay sólo la reenvía si el host coincide con la solicitud pendiente, que expira en 60 segundos.

El protocolo actual **no transporta video ni input**. Es deliberado: no debe usarse para transmitir datos de sesión sin agregar autenticación mutua, cifrado E2E, nonces y contadores de secuencia.
