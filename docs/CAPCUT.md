# Catálogo de CapCut

openCUP puede mostrar los paneles reales de CapCut (efectos, transiciones, filtros,
stickers y plantillas de texto) con sus nombres, categorías originales y portadas.

## De dónde sale

CapCut de escritorio guarda en disco todas las respuestas de su plataforma de efectos:

```
%LOCALAPPDATA%\CapCut\User Data\Cache\ressdk_db\<id>\rp.db   (SQLite, tabla http_cache)
```

Cada fila es una respuesta JSON de `get_panel_info` (categorías, en su orden) o de
`get_resources_by_category_id` (ítems con portada, id, md5 y URL del paquete `.zip`).

Los paquetes descargados se descomprimen en `Cache\effect\<id>\` e incluyen
`config.json`, escena `.prefab`, materiales, shaders GLSL y scripts Lua (`Amaz`).

## Flujo de trabajo

```bash
# 1. Leer la caché de CapCut y construir el catálogo normalizado
node tools/capcut/build-catalog.mjs          # -> catalog/capcut-catalog.json

# 2. Agrupar por panel de openCUP y conservar el orden de categorías de CapCut
node tools/capcut/build-panels.mjs           # -> catalog/panels.json

# 3. Descargar las portadas
node tools/capcut/download-panel-covers.mjs  # -> capcut-dl/covers-panels/<panel>/<id>.webp

# 4. Generar el módulo que consume la app
node tools/capcut/build-panel-module.mjs     # -> capcut-panels.js (src/)
```

Los scripts llevan rutas absolutas de la caché de CapCut y de la carpeta de descargas;
ajústalas si mueves el proyecto.

## Qué se versiona y qué no

- `src/capcut-panels.js` (catálogo con nombres, categorías y URLs de portada) **sí** se
  versiona: es texto y pesa unos 3 MB.
- `assets/capcut/` (portadas, ~1,9 GB) **no** se versiona. La app usa la portada local si
  existe y, si no, la URL firmada de la CDN de CapCut.

## Shaders

`src/gl-post.js` es el runner WebGL. Cada shader es un porte de un `.frag` real de un
paquete de CapCut. La nomenclatura de uniformes se mantiene (`u_time`, `u_amount`,
`u_res`) para que portar uno nuevo sea copiar el `.frag` y añadir su entrada en
`NAME_MAP`.
