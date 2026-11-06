Add-Type @"
using System;using System.Text;using System.Runtime.InteropServices;
public class Dlg32 {
 [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr lp);
 public delegate bool EnumProc(IntPtr h, IntPtr lp);
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint msg, IntPtr w, IntPtr l);
 public static void CerrarDialogos(uint pid) {
   EnumWindows((h, lp) => {
     uint p; GetWindowThreadProcessId(h, out p);
     if (p != pid) return true;
     var cls = new StringBuilder(64); GetClassName(h, cls, 64);
     if (cls.ToString() == "#32770") PostMessage(h, 0x0010, IntPtr.Zero, IntPtr.Zero); // WM_CLOSE
     return true;
   }, IntPtr.Zero);
 }
}
"@

# 1) Cierre limpio: la ventana debe matar a node.
$p = Get-Process cupcraft -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if ($p) { $p.CloseMainWindow() | Out-Null; Start-Sleep -Seconds 4 }
$rest = Get-Process cupcraft -ErrorAction SilentlyContinue
Write-Output ("tras cerrar: procesos cupcraft=" + $(if ($rest) { $rest.Count } else { 0 }))
$nodeReciente = Get-Process node -ErrorAction SilentlyContinue | Where-Object { $_.StartTime -gt (Get-Date).AddSeconds(-120) }
Write-Output ("node reciente=" + $(if ($nodeReciente) { $nodeReciente.Count } else { 0 }))
Write-Output ("puerto 8787: " + $(if (netstat -ano | Select-String ':8787.*LISTENING') { "ocupado" } else { "libre" }))

# 2) Instancia única: el segundo arranque debe mostrar el aviso y salir.
Start-Process -FilePath (Resolve-Path dist\cupcraft\cupcraft.exe).Path
Start-Sleep -Seconds 9
$uno = Get-Process cupcraft -ErrorAction SilentlyContinue
Write-Output ("instancia 1: PID " + (($uno | ForEach-Object { $_.Id }) -join ','))
$segundo = Start-Process -FilePath (Resolve-Path dist\cupcraft\cupcraft.exe).Path -PassThru
Start-Sleep -Seconds 4
$todos = @(Get-Process cupcraft -ErrorAction SilentlyContinue)
Write-Output ("procesos tras 2º arranque: " + $todos.Count)
Write-Output ("ventanas: " + (($todos | ForEach-Object { "'" + $_.MainWindowTitle + "'" }) -join ' | '))
[Dlg32]::CerrarDialogos([uint32]$segundo.Id)
Start-Sleep -Seconds 3
$despues = @(Get-Process cupcraft -ErrorAction SilentlyContinue)
Write-Output ("tras descartar aviso: procesos=" + $despues.Count + " (el 2º debe haber salido)")
