# Genera src/logo.png y src/favicon.png a partir de la imagen del usuario.
# Recorta el cuadrado redondeado, lo escala a 256px y aplica esquinas
# transparentes para que el logo se vea bien sobre la interfaz oscura.
param(
  [string]$Source = "$env:USERPROFILE\Pictures\41cd1709-d22e-4dda-bd20-f0e0a2e63d9e.png",
  [string]$OutLogo = "",
  [string]$OutFavicon = ""
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $OutLogo) { $OutLogo = Join-Path $root 'src\logo.png' }
if (-not $OutFavicon) { $OutFavicon = Join-Path $root 'src\favicon.png' }
if (!(Test-Path $Source)) { throw "No se encontró la imagen de origen: $Source" }

Add-Type -AssemblyName System.Drawing

$src = [System.Drawing.Bitmap]::FromFile($Source)
try {
  $w = $src.Width; $h = $src.Height

  # 1. Bounding box del contenido que no es negro (el cuadrado del logo).
  $minX = $w; $maxX = -1; $minY = $h; $maxY = -1
  for ($y = 0; $y -lt $h; $y += 2) {
    for ($x = 0; $x -lt $w; $x += 2) {
      $p = $src.GetPixel($x, $y)
      if ($p.R -gt 24 -or $p.G -gt 24 -or $p.B -gt 24) {
        if ($x -lt $minX) { $minX = $x }
        if ($x -gt $maxX) { $maxX = $x }
        if ($y -lt $minY) { $minY = $y }
        if ($y -gt $maxY) { $maxY = $y }
      }
    }
  }
  if ($maxX -lt 0) { throw 'La imagen de origen no contiene contenido visible.' }

  # 2. Cuadrado perfecto centrado en ese bounding box.
  $bw = $maxX - $minX + 1
  $bh = $maxY - $minY + 1
  $side = [Math]::Max($bw, $bh)
  $cx = [int](($minX + $maxX) / 2)
  $cy = [int](($minY + $maxY) / 2)
  $sx = [Math]::Max(0, $cx - [int]($side / 2))
  $sy = [Math]::Max(0, $cy - [int]($side / 2))
  if ($sx + $side -gt $w) { $sx = $w - $side }
  if ($sy + $side -gt $h) { $sy = $h - $side }

  $cropRect = New-Object System.Drawing.Rectangle $sx, $sy, $side, $side
  $cropped = $src.Clone($cropRect, $src.PixelFormat)

  # 3. Función de recorte redondeado + escala a un tamaño dado.
  function New-RoundedLogo([System.Drawing.Bitmap]$image, [int]$size, [double]$radiusRatio) {
    $radius = [Math]::Max(1, [int]($size * $radiusRatio))
    $bmp = [System.Drawing.Bitmap]::new($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.Clear([System.Drawing.Color]::Transparent)

    $diameter = $radius * 2
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddArc(0, 0, $diameter, $diameter, 180, 90)
    $path.AddArc($size - $diameter, 0, $diameter, $diameter, 270, 90)
    $path.AddArc($size - $diameter, $size - $diameter, $diameter, $diameter, 0, 90)
    $path.AddArc(0, $size - $diameter, $diameter, $diameter, 90, 90)
    $path.CloseFigure()

    $g.SetClip($path)
    $g.DrawImage($image, 0, 0, $size, $size)
    $g.Dispose()
    $path.Dispose()
    return $bmp
  }

  # 24% ligeramente mayor que el radio real (22.5%) para no dejar restos negros.
  $logo = New-RoundedLogo $cropped 256 0.24
  $dir = Split-Path $OutLogo
  if (!(Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
  $logo.Save($OutLogo, [System.Drawing.Imaging.ImageFormat]::Png)
  $logo.Dispose()

  $favicon = New-RoundedLogo $cropped 64 0.24
  $fdir = Split-Path $OutFavicon
  if (!(Test-Path $fdir)) { New-Item -ItemType Directory -Force -Path $fdir | Out-Null }
  $favicon.Save($OutFavicon, [System.Drawing.Imaging.ImageFormat]::Png)
  $favicon.Dispose()

  $cropped.Dispose()
  Write-Output "logo: $OutLogo (256x256)"
  Write-Output "favicon: $OutFavicon (64x64)"
} finally {
  $src.Dispose()
}
