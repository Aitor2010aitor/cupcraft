param(
  [string]$Out = "C:\Users\aitor\Desktop\cupcraft\build\openCUP.ico",
  [string]$Logo = ""
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $Logo) { $Logo = Join-Path $root 'src\logo.png' }
if (!(Test-Path $Logo)) { throw "No se encontró el logo: $Logo (ejecuta tools\make-logo.ps1 antes)" }

Add-Type -AssemblyName System.Drawing

$sizes = @(16, 24, 32, 48, 64, 128, 256)
# El maestro es el logo generado (256x256 con esquinas redondeadas transparentes).
$master = [System.Drawing.Bitmap]::new(256, 256, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($master)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
$g.Clear([System.Drawing.Color]::Transparent)
$src = [System.Drawing.Bitmap]::FromFile($Logo)
$g.DrawImage($src, 0, 0, 256, 256)
$src.Dispose()
$g.Dispose()

$images = @()
foreach ($size in $sizes) {
  $bmp = [System.Drawing.Bitmap]::new($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $ctx = [System.Drawing.Graphics]::FromImage($bmp)
  $ctx.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $ctx.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $ctx.Clear([System.Drawing.Color]::Transparent)
  $ctx.DrawImage($master, 0, 0, $size, $size)
  $ctx.Dispose()
  $stream = New-Object System.IO.MemoryStream
  $bmp.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
  $images += ,$stream.ToArray()
  $stream.Dispose()
  $bmp.Dispose()
}
$master.Dispose()

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Out) | Out-Null
$file = [System.IO.File]::Create($Out)
$writer = New-Object System.IO.BinaryWriter $file
$writer.Write([UInt16]0)
$writer.Write([UInt16]1)
$writer.Write([UInt16]$sizes.Count)
$offset = 6 + (16 * $sizes.Count)
for ($index = 0; $index -lt $sizes.Count; $index++) {
  $size = $sizes[$index]
  $bytes = $images[$index]
  $writer.Write([Byte]$(if ($size -ge 256) { 0 } else { $size }))
  $writer.Write([Byte]$(if ($size -ge 256) { 0 } else { $size }))
  $writer.Write([Byte]0)
  $writer.Write([Byte]0)
  $writer.Write([UInt16]1)
  $writer.Write([UInt16]32)
  $writer.Write([UInt32]$bytes.Length)
  $writer.Write([UInt32]$offset)
  $offset += $bytes.Length
}
foreach ($bytes in $images) { $writer.Write($bytes) }
$writer.Flush()
$writer.Dispose()
$file.Dispose()
Write-Output "icon -> $Out ($($sizes.Count) sizes)"
