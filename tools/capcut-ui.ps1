param(
  [Parameter(Mandatory=$true)][string]$Action,
  [int]$X = 0,
  [int]$Y = 0,
  [int]$Delta = 0,
  [int]$Width = 3440,
  [int]$Height = 1392,
  [switch]$KeepOnScreen,
  [string]$Out = "C:\Users\aitor\Desktop\cupcraft\shots\capcut-ui.png"
)

Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public class CapUI {
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern int GetClassName(IntPtr hWnd, StringBuilder lpClassName, int nMaxCount);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
  [DllImport("user32.dll")] public static extern bool GetClientRect(IntPtr hWnd, out RECT lpRect);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdcBlt, uint nFlags);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr after, int X, int Y, int cx, int cy, uint flags);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int X, int Y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
'@

function Get-CapMainWindow {
  # Prefer the process main window: it stays stable even when CapCut hides
  # secondary Qt windows during project loading.
  $proc = Get-Process CapCut -ErrorAction SilentlyContinue |
    Where-Object { $_.MainWindowHandle -ne 0 } |
    Sort-Object WorkingSet64 -Descending |
    Select-Object -First 1
  if ($proc) { return [IntPtr]$proc.MainWindowHandle }

  $script:best = [IntPtr]::Zero
  $script:bestArea = 0
  $cb = [CapUI+EnumWindowsProc]{
    param($h, $l)
    $procId = [uint32]0
    [CapUI]::GetWindowThreadProcessId($h, [ref]$procId) | Out-Null
    $p = Get-Process -Id $procId -ErrorAction SilentlyContinue
    if ($p -and $p.ProcessName -eq 'CapCut') {
      $cn = New-Object System.Text.StringBuilder 256
      [CapUI]::GetClassName($h, $cn, 256) | Out-Null
      if ($cn.ToString() -like 'Qt622QWindowIcon*' -or $cn.ToString() -like 'MainWindow*') {
        $r = New-Object CapUI+RECT
        [CapUI]::GetWindowRect($h, [ref]$r) | Out-Null
        $area = ($r.Right - $r.Left) * ($r.Bottom - $r.Top)
        if ($area -gt $script:bestArea) { $script:bestArea = $area; $script:best = $h }
      }
    }
    return $true
  }
  [CapUI]::EnumWindows($cb, [IntPtr]::Zero) | Out-Null
  return $script:best
}

$h = Get-CapMainWindow
if ($h -eq [IntPtr]::Zero) { Write-Output "ERROR: CapCut window not found"; exit 1 }

function Ensure-CapWindow {
  if ([CapUI]::IsIconic($h)) {
    [CapUI]::ShowWindow($h, 4) | Out-Null
    Start-Sleep -Milliseconds 400
  }
  $r = New-Object CapUI+RECT
  [CapUI]::GetWindowRect($h, [ref]$r) | Out-Null
  $w = $r.Right - $r.Left; $ht = $r.Bottom - $r.Top
  $x = if ($KeepOnScreen) { -8 } else { -$Width }
  if ($w -ne $Width -or $ht -ne $Height -or $r.Left -ne $x) {
    # SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOSENDCHANGING: resize and park
    # off-screen without stealing focus from the user's current app.
    [CapUI]::SetWindowPos($h, [IntPtr]::Zero, $x, 0, $Width, $Height, 0x0414) | Out-Null
    Start-Sleep -Milliseconds 700
  }
}

Ensure-CapWindow

switch ($Action) {
  'capture' {
    $r = New-Object CapUI+RECT
    [CapUI]::GetWindowRect($h, [ref]$r) | Out-Null
    $w = $r.Right - $r.Left; $ht = $r.Bottom - $r.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $ht
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $hdc = $g.GetHdc()
    $ok = [CapUI]::PrintWindow($h, $hdc, 2)
    $g.ReleaseHdc($hdc)
    $g.Dispose()
    $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Output "captured hwnd=0x$('{0:X}' -f $h.ToInt64()) ${w}x${ht} ok=$ok -> $Out"
  }
  'click' {
    $lParam = [IntPtr](($Y -shl 16) -bor ($X -band 0xFFFF))
    [CapUI]::SendMessage($h, 0x0200, [IntPtr]0, $lParam) | Out-Null  # WM_MOUSEMOVE
    Start-Sleep -Milliseconds 120
    [CapUI]::SendMessage($h, 0x0201, [IntPtr]1, $lParam) | Out-Null   # WM_LBUTTONDOWN
    Start-Sleep -Milliseconds 90
    [CapUI]::SendMessage($h, 0x0202, [IntPtr]0, $lParam) | Out-Null   # WM_LBUTTONUP
    Start-Sleep -Milliseconds 600
    Write-Output "clicked ($X,$Y) on hwnd=0x$('{0:X}' -f $h.ToInt64())"
  }
  'clickcap' {
    $lParam = [IntPtr](($Y -shl 16) -bor ($X -band 0xFFFF))
    [CapUI]::SendMessage($h, 0x0200, [IntPtr]0, $lParam) | Out-Null
    Start-Sleep -Milliseconds 120
    [CapUI]::SendMessage($h, 0x0201, [IntPtr]1, $lParam) | Out-Null
    Start-Sleep -Milliseconds 90
    [CapUI]::SendMessage($h, 0x0202, [IntPtr]0, $lParam) | Out-Null
    Start-Sleep -Milliseconds 1600
    $r = New-Object CapUI+RECT
    [CapUI]::GetWindowRect($h, [ref]$r) | Out-Null
    $w = $r.Right - $r.Left; $ht = $r.Bottom - $r.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $ht
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $hdc = $g.GetHdc()
    [CapUI]::PrintWindow($h, $hdc, 2) | Out-Null
    $g.ReleaseHdc($hdc)
    $g.Dispose()
    $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    Write-Output "clicked ($X,$Y) + captured ${w}x${ht} -> $Out"
  }
  'hover' {
    $lParam = [IntPtr](($Y -shl 16) -bor ($X -band 0xFFFF))
    [CapUI]::PostMessage($h, 0x0200, [IntPtr]0, $lParam) | Out-Null
    Write-Output "hover ($X,$Y)"
  }
  'realclick' {
    [CapUI]::SetWindowPos($h, [IntPtr](-1), 0, 0, 0, 0, 0x0003 -bor 0x0010) | Out-Null
    Start-Sleep -Milliseconds 250
    $r = New-Object CapUI+RECT
    [CapUI]::GetWindowRect($h, [ref]$r) | Out-Null
    [CapUI]::SetCursorPos($r.Left + $X, $r.Top + $Y) | Out-Null
    Start-Sleep -Milliseconds 200
    [CapUI]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 90
    [CapUI]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 700
    [CapUI]::SetWindowPos($h, [IntPtr](-2), 0, 0, 0, 0, 0x0003 -bor 0x0010) | Out-Null
    Write-Output "realclick ($X,$Y)"
  }
  'realhover' {
    [CapUI]::SetWindowPos($h, [IntPtr](-1), 0, 0, 0, 0, 0x0003 -bor 0x0010) | Out-Null
    Start-Sleep -Milliseconds 250
    $r = New-Object CapUI+RECT
    [CapUI]::GetWindowRect($h, [ref]$r) | Out-Null
    [CapUI]::SetCursorPos($r.Left + $X, $r.Top + $Y) | Out-Null
    Start-Sleep -Milliseconds 1500
    Write-Output "realhover ($X,$Y)"
  }
  'unpin' {
    [CapUI]::SetWindowPos($h, [IntPtr](-2), 0, 0, 0, 0, 0x0003 -bor 0x0010) | Out-Null
    Write-Output "unpinned"
  }
  'wheel' {
    $lParam = [IntPtr](($Y -shl 16) -bor ($X -band 0xFFFF))
    $wParam = [IntPtr](($Delta -shl 16) -band 0xFFFFFFFF)
    [CapUI]::PostMessage($h, 0x020A, $wParam, $lParam) | Out-Null
    Start-Sleep -Milliseconds 500
    Write-Output "wheel delta=$Delta at ($X,$Y)"
  }
  default { Write-Output "unknown action $Action" }
}
