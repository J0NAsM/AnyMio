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
  "url": "https://github.com/J0NAsM/AnyMio/releases/download/v0.2.0/JRemote.exe",
  "sha256": "HASH_SHA256_DE_64_CARACTERES_DEL_JRemote.exe",
  "notes": "Correcciones y mejoras de estabilidad."
}
```

La versión debe seguir el formato `mayor.menor.parche` y la URL de descarga debe
usar HTTPS. Para una versión más nueva, `sha256` es obligatorio y debe contener
el hash SHA-256, en hexadecimal, de `JRemote.exe`.

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

Para instalar una actualización ya detectada, ejecuta:

```powershell
.\JRemote.exe --install-update
```

`JRemoteUpdater.exe` descarga el archivo por HTTPS, limita la descarga a 200
MiB, comprueba el SHA-256 y solo entonces sustituye `JRemote.exe`. Conserva una
copia previa como `JRemote.previous.exe` para recuperación manual si fuera
necesario. El checksum protege la transferencia y errores de publicación, pero
no sustituye la firma Authenticode: esta debe añadirse cuando haya un certificado.

## Publicación automatizada

El workflow `.github/workflows/release.yml` se ejecuta al subir una etiqueta
`vX.Y.Z`. Exige que coincida con `Cargo.toml`, construye ambos ejecutables y el
instalador NSIS, crea el Release y actualiza el manifiesto con el hash real. El
script local `scripts/Prepare-Release.ps1` ofrece el mismo preparado antes de
crear una etiqueta manualmente.

## Canales

El canal **Estable** consulta `update.json`; el canal **Beta** consulta
`update-beta.json`. La preferencia se guarda localmente y la interfaz permite
seleccionarla. Ambos manifiestos siguen exigiendo HTTPS y SHA-256 para una
versión que sea más nueva que la instalada.
