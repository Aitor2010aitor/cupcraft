Add-Type @"
using System;using System.Text;using System.Collections.Generic;using System.Runtime.InteropServices;
public class Dump32 {
 [DllImport("user32.dll")] public static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string cls, string title);
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr h, StringBuilder s, int n);
 [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
 [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
 public static List<string> Walk(IntPtr root) {
   var list = new List<string>();
   var stack = new Stack<IntPtr>(); stack.Push(root);
   while (stack.Count > 0) {
     IntPtr cur = stack.Pop();
     IntPtr child = FindWindowEx(cur, IntPtr.Zero, null, null);
     while (child != IntPtr.Zero) {
       var cls = new StringBuilder(256); GetClassName(child, cls, 256);
       var txt = new StringBuilder(256); GetWindowText(child, txt, 256);
       RECT r; GetWindowRect(child, out r);
       list.Add(string.Format("{0} | visible={1} | {2}x{3} | texto='{4}'",
         cls.ToString(), IsWindowVisible(child), (r.R-r.L), (r.B-r.T), txt.ToString()));
       stack.Push(child);
       child = FindWindowEx(cur, child, null, null);
     }
   }
   return list;
 }
}
"@
$p = Get-Process cupcraft -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $p) { Write-Output "SIN VENTANA"; exit 1 }
Write-Output ("ventana principal handle=" + $p.MainWindowHandle + " titulo='" + $p.MainWindowTitle + "'")
$kids = [Dump32]::Walk($p.MainWindowHandle)
Write-Output ("hijos: " + $kids.Count)
$kids | ForEach-Object { Write-Output ("  " + $_) }
