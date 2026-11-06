Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;using System.Text;using System.Runtime.InteropServices;
public class Cap32 {
 [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
 [DllImport("user32.dll")] public static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string cls, string title);
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
 public const uint PW_RENDERFULLCONTENT = 2;
}
"@

$p = Get-Process cupcraft -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { Write-Output "SIN VENTANA"; exit 1 }
$hwnd = $p.MainWindowHandle

# 1) Hijos de la ventana: etiqueta de estado + renderizador WebView2.
$estado = [Cap32]::FindWindowEx($hwnd, [IntPtr]::Zero, "STATIC", $null)
$estadoTxt = New-Object System.Text.StringBuilder 256
if ($estado -ne [IntPtr]::Zero) { [Cap32]::GetWindowText($estado, $estadoTxt, 256) | Out-Null }
$render = [Cap32]::FindWindowEx($hwnd, [IntPtr]::Zero, "Chrome_RenderWidgetHostHWND", $null)

$vis = "no existe"
if ($estado -ne [IntPtr]::Zero) { $vis = [Cap32]::IsWindowVisible($estado) }
Write-Output ("etiqueta estado: '" + $estadoTxt.ToString() + "' visible=" + $vis)
Write-Output ("renderizador WebView2: " + $(if ($render -ne [IntPtr]::Zero) { "SI" } else { "NO" }))

# 2) Captura de la propia ventana aunque esté tapada (PrintWindow).
$rect = New-Object Cap32+RECT
[Cap32]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
$w = $rect.R - $rect.L; $h = $rect.B - $rect.T
$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
[Cap32]::PrintWindow($hwnd, $hdc, [Cap32]::PW_RENDERFULLCONTENT) | Out-Null
$g.ReleaseHdc($hdc); $g.Dispose()
$out = "$env:TEMP\cupcraft_ventana.png"
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output "captura $w x $h -> $out"
