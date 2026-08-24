# Dependencias

| Crate | Licencia | Uso | Alternativa considerada |
|---|---|---|---|
| Tokio | MIT | Listener TCP y concurrencia del relay | async-std |
| Serde / serde_json | MIT/Apache-2.0 | Mensajes explícitos auditables | Protobuf (se reserva para datos de sesión) |
| Argon2 | MIT/Apache-2.0 | Argon2id para hashes de contraseñas | scrypt |
| UUID | MIT/Apache-2.0 | UUID v4 de identidad y solicitudes | implementación propia (descartada) |
| Clap | MIT/Apache-2.0 | CLI `--relay`, `--port`, `--version` | parsing manual |
| Directories | MIT/Apache-2.0 | Directorio de datos de usuario | rutas manuales de Windows |
| Tracing | MIT | Diagnóstico local estructurado | log |

Las licencias deben volver a revisarse al incorporar captura, codecs y transporte de sesión. En particular, el uso y distribución de H.264/Media Foundation requiere una revisión legal y de patentes antes de publicar el producto.
