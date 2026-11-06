$ErrorActionPreference = 'Continue'
$root = Split-Path -Parent $PSScriptRoot

# Limpieza: retira procesos antiguos que ocupen los puertos de prueba.
Get-Process cupcraft -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
foreach ($port in 8787, 8790) {
  $linea = netstat -ano | Select-String (":$port\s.*LISTENING")
  foreach ($l in $linea) {
    $procId = ($l.ToString().Trim() -split '\s+')[-1]
    if ($procId -match '^\d+$') { Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue }
  }
}
Start-Sleep -Seconds 2

$server = Start-Process -FilePath 'node' -ArgumentList 'server.mjs' -WorkingDirectory $root -PassThru -WindowStyle Hidden
Start-Sleep -Seconds 5
try {
  $r = Invoke-WebRequest -Uri 'http://127.0.0.1:8787/' -UseBasicParsing -TimeoutSec 8
  Write-Output ("servidor: HTTP " + $r.StatusCode)
} catch {
  Write-Output ("servidor NO arrancó: " + $_.Exception.Message)
}

$suites = @(
  @{ name = 'functional';      cmd = 'node tools/functional-test.mjs' },
  @{ name = 'layout';          cmd = 'node tools/layout-test.mjs' },
  @{ name = 'features';        cmd = 'node tools/features-test.mjs' },
  @{ name = 'integrations';    cmd = 'node tools/new-features-test.mjs' },
  @{ name = 'voice';           cmd = 'node tools/voice-test.mjs' },
  @{ name = 'export';          cmd = 'node tools/export-test.mjs' },
  @{ name = 'audio-export';    cmd = 'node tools/audio-export-test.mjs' },
  @{ name = 'visual';          cmd = 'node tools/visual-test.mjs' },
  @{ name = 'preview-drag';    cmd = 'node tools/preview-drag-test.mjs' },
  @{ name = 'effects-visual';  cmd = 'node tools/effects-visual-test.mjs' },
  @{ name = 'text-panel';      cmd = 'node tools/text-panel-test.mjs' },
  @{ name = 'project-features'; cmd = 'node tools/project-features-test.mjs' }
)

$ok = 0; $ko = 0; $resumen = @()
foreach ($s in $suites) {
  $out = & cmd /c "cd /d `"$root`" && $($s.cmd) 2>&1"
  $code = $LASTEXITCODE
  $tail = ($out | Select-Object -Last 4) -join ' | '
  if ($code -eq 0) { $ok++; $estado = 'PASS' } else { $ko++; $estado = 'FAIL' }
  $resumen += ("[{0}] {1} :: {2}" -f $estado, $s.name, $tail)
  Write-Output ("[{0}] {1}" -f $estado, $s.name)
}

Write-Output ""
Write-Output "===== RESUMEN ====="
$resumen | ForEach-Object { Write-Output $_ }
Write-Output ("TOTAL: " + $ok + " pass / " + $ko + " fail")
if ($server -and -not $server.HasExited) { Stop-Process -Id $server.Id -Force }
if ($ko -gt 0) { exit 1 }
