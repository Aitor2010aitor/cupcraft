# cupcraft en C# / .NET

Migración del backend y del lanzador a **C# con .NET 8**. La interfaz web (`src/`)
sigue siendo JavaScript por ahora: portarla módulo a módulo es la siguiente fase.

## Estructura

```
dotnet/
├─ OpenCup.Server/      Servidor Kestrel (ASP.NET Core) que sirve la app y la API
│  ├─ OpenCup.Server.csproj
│  └─ ServerApp.cs
├─ OpenCup.Desktop/     App de escritorio WinForms + WebView2 (sustituye al launcher C++)
│  ├─ OpenCup.Desktop.csproj
│  └─ Program.cs
└─ NuGet.config         Fuente nuget.org (permite compilar sin tocar el perfil)
```

## Qué hace ya en C#

| Pieza | Estado |
|---|---|
| Servidor estático (`index.html`, `src/`, `assets/`) | ✅ Kestrel |
| `/api/health` y `/api/version` | ✅ |
| Ventana de escritorio con WebView2 | ✅ (WinForms) |
| Elección automática de puerto (8787-8800) | ✅ |
| Modo `--server-only` para pruebas sin ventana | ✅ |
| `/api/capcut/*` (borradores de CapCut) | ⏳ pendiente, sigue en `server.mjs` |

## Compilar

```powershell
dotnet build dotnet/OpenCup.Desktop/OpenCup.Desktop.csproj -c Release
```

Salida: `dotnet/OpenCup.Desktop/bin/Release/net8.0-windows/cupcraft.exe`.

Si el usuario no tiene permisos en `%APPDATA%`, define un `NuGet.config` propio
(ya incluido) y compila con `-p:RestoreConfigFile=NuGet.config`.

## Probar

```powershell
.\cupcraft.exe --server-only      # solo servidor (sin ventana)
curl http://127.0.0.1:8787/api/health
```

## Siguiente fase

1. Portar `/api/capcut/*` (raíces, listado e importación de borradores) a C#.
2. Portar los módulos de UI: `store` → servicios, `renderer` → `System.Drawing`/SkiaSharp,
   `panels` → controles WinForms.
3. Sustituir el runner WebGL por un render con GPU nativa (Vortice/Silk.NET).
