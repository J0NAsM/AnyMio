# Build

La compilación de Windows usa `stable-x86_64-pc-windows-msvc`, por lo que requiere el workload **Desktop development with C++** / `Microsoft.VisualStudio.Workload.VCTools` de Visual Studio Build Tools. Git for Windows incluye otro `link.exe` incompatible y no basta para Cargo.

Tras instalarlo, abra una consola de desarrollador de Visual Studio o ejecute `vcvars64.bat` antes de llamar a Cargo.

La compilación de release genera `target\release\JRemote.exe`. La copia portable para pruebas se publica en `dist\JRemote.exe`; incluye la interfaz local, el relay de señalización y el actualizador, pero no captura de pantalla, viewer ni inyección de entrada remota.

La compilación también genera `target\release\JRemoteUpdater.exe`; ambos
archivos deben distribuirse juntos para habilitar la instalación de una
actualización descargada y verificada.

## Instalador Windows

El instalador básico para usuario actual está en `installer/AnyMio.nsi`. Con
NSIS instalado y los binarios de release generados:

```powershell
makensis /DVERSION=0.2.4 installer/AnyMio.nsi
```

Esto produce `installer/AnyMio-Setup-0.2.4.exe`. El workflow de Release lo
genera automáticamente para las etiquetas de versión.
