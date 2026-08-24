# Build

La compilación de Windows usa `stable-x86_64-pc-windows-msvc`, por lo que requiere el workload **Desktop development with C++** / `Microsoft.VisualStudio.Workload.VCTools` de Visual Studio Build Tools. Git for Windows incluye otro `link.exe` incompatible y no basta para Cargo.

Tras instalarlo, abra una consola de desarrollador de Visual Studio o ejecute `vcvars64.bat` antes de llamar a Cargo.

La compilación de release genera `target\release\JRemote.exe`. La copia portable para pruebas se publica en `dist\JRemote.exe`; es sólo el relay/señalización, porque la aplicación de escritorio no está completa.
