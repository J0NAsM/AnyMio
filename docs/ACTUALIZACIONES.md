# Actualizaciones

JRemote consulta el manifiesto público de este repositorio al abrirse. Si la
versión publicada es mayor que la integrada en el ejecutable, muestra un mensaje
con el enlace de descarga. La comprobación usa HTTPS, tiene un límite de tres
segundos y un fallo de red nunca impide abrir la aplicación.

El repositorio debe ser **público** para que los equipos de los usuarios puedan
leer `update.json` sin una cuenta de GitHub. Si prefieres mantener el código
privado, publica ese archivo y los instaladores en otro hosting HTTPS público.

## Publicar una versión

1. Incrementa la versión en `Cargo.toml`, compila `cargo build --release` y
   publica `target\\release\\JRemote.exe` en GitHub Releases con una etiqueta
   como `v0.2.0`.
2. Actualiza `update.json` en la rama `main` con este contenido:

```json
{
  "version": "0.2.0",
  "url": "https://github.com/USUARIO/JRemote/releases/download/v0.2.0/JRemote.exe",
  "notes": "Correcciones y mejoras de estabilidad."
}
```

La versión debe seguir el formato `mayor.menor.parche` y la URL de descarga debe
usar HTTPS.

## Integrar el manifiesto en la versión publicada

Antes de compilar la versión que distribuirás, define su URL definitiva:

```powershell
$env:JREMOTE_UPDATE_MANIFEST_URL = "https://tu-dominio.example/update.json"
cargo build --release
```

Esa URL sustituye el manifiesto predeterminado de AnyMio y sirve para una versión
privada o una bifurcación del proyecto. Al abrirlo, todos los usuarios recibirán
el aviso cuando publiques una versión mayor.

## Probar otra URL

Para probarlo en una consola de Windows:

```powershell
$env:JREMOTE_UPDATE_MANIFEST_URL = "https://tu-dominio.example/update.json"
.\dist\JRemote.exe
```

O usa una sola vez el argumento `--update-manifest-url`:

```powershell
.\dist\JRemote.exe --update-manifest-url "https://tu-dominio.example/update.json"
```

Esta primera versión abre una ruta de descarga en vez de reemplazar el ejecutable
en uso; para actualizar de forma automática se necesita un proceso actualizador
independiente y firma de binarios.
