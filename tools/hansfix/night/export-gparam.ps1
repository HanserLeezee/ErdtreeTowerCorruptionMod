# Extracts a few lighting (gparam) files from Elden Ring's Data*.bdt for the HansFix patch. Read-only.
$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
$game = Join-Path (Split-Path $here -Parent) 'Game'
$out = Join-Path $here 'gparam-export'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$log = Join-Path $out 'export-log.txt'
"start $(Get-Date -Format o) game=$game" | Set-Content -Encoding ASCII $log
Add-Type -TypeDefinition @"
using System; using System.Runtime.InteropServices;
public static class Oodle {
  [DllImport("oo2core_6_win64.dll")] public static extern long OodleLZ_Decompress(byte[] src, long srcLen, byte[] dst, long dstLen, int fuzz, int crc, int verb, IntPtr b, long bs, IntPtr cb, IntPtr cbd, IntPtr mem, long ms, int phase);
}
"@
[System.Environment]::CurrentDirectory = $game
$env:PATH = "$game;$env:PATH"
$items = @(
  @{Name='m00_00_0000.gparam.dcx';Archive='Data0';Offset=[int64]103485280;Size=3792;Key='d04dc3507fd372f452f1c5cd03151c34';Ranges='0-3792'},
  @{Name='m00_00_0000_weatherbase.gparam.dcx';Archive='Data0';Offset=[int64]103492528;Size=832;Key='d04dc3507fd372f452f1c5cd03151c34';Ranges='0-832'},
  @{Name='default.gparam.dcx';Archive='Data0';Offset=[int64]103476992;Size=6416;Key='d04dc3507fd372f452f1c5cd03151c34';Ranges='0-6416'},
  @{Name='m00_00_0000_weatheroutdoor.gparam.dcx';Archive='Data0';Offset=[int64]103493360;Size=704;Key='d04dc3507fd372f452f1c5cd03151c34';Ranges='0-704'},
  @{Name='m60_00_0000.gparam.dcx';Archive='Data0';Offset=[int64]6350131898;Size=4688;Key='d04dc3507fd372f452f1c5cd03151c34';Ranges='0-4688'},
  @{Name='m45_02_0000.gparam.dcx';Archive='Data0';Offset=[int64]11356407635;Size=4160;Key='d04dc3507fd372f452f1c5cd03151c34';Ranges='0-4160'},
  @{Name='m45_01_0000.gparam.dcx';Archive='Data0';Offset=[int64]11356404067;Size=3568;Key='d04dc3507fd372f452f1c5cd03151c34';Ranges='0-3568'},
  @{Name='m45_00_0000.gparam.dcx';Archive='Data0';Offset=[int64]11356399683;Size=4384;Key='d04dc3507fd372f452f1c5cd03151c34';Ranges='0-4384'}
)
function HexBytes($h){ $b = New-Object byte[] ($h.Length/2); for($i=0;$i -lt $b.Length;$i++){ $b[$i]=[Convert]::ToByte($h.Substring($i*2,2),16) }; $b }
function Undcx($b){
  if($b[0] -ne 0x44 -or $b[1] -ne 0x43 -or $b[2] -ne 0x58){ return $null }
  $us = ([int64]$b[0x1c] -shl 24) -bor ([int64]$b[0x1d] -shl 16) -bor ([int64]$b[0x1e] -shl 8) -bor [int64]$b[0x1f]
  $cs = ([int64]$b[0x20] -shl 24) -bor ([int64]$b[0x21] -shl 16) -bor ([int64]$b[0x22] -shl 8) -bor [int64]$b[0x23]
  $src = New-Object byte[] $cs; [Array]::Copy($b, 0x4c, $src, 0, $cs)
  $dst = New-Object byte[] ($us + 64)
  $n = [Oodle]::OodleLZ_Decompress($src, $cs, $dst, $us, 1, 0, 0, [IntPtr]::Zero, 0, [IntPtr]::Zero, [IntPtr]::Zero, [IntPtr]::Zero, 0, 3)
  if($n -ne $us){ return $null }
  $r = New-Object byte[] $us; [Array]::Copy($dst, $r, $us); $r
}
foreach($it in $items){
  try {
    $bdt = Join-Path $game ($it.Archive + '.bdt')
    $fs = [System.IO.File]::OpenRead($bdt)
    $buf = New-Object byte[] $it.Size
    $fs.Seek($it.Offset, 'Begin') | Out-Null
    $got = $fs.Read($buf, 0, $it.Size); $fs.Close()
    if($it.Key){
      $aes = [System.Security.Cryptography.Aes]::Create(); $aes.Mode='ECB'; $aes.Padding='None'; $aes.Key = HexBytes $it.Key
      $dec = $aes.CreateDecryptor()
      foreach($r in $it.Ranges.Split(';')){ if(-not $r){continue}; $p=$r.Split('-'); $a=[int]$p[0]; $e=[int]$p[1]; if($e -gt $a){ $dec.TransformBlock($buf,$a,$e-$a,$buf,$a) | Out-Null } }
    }
    $dcx = Join-Path $out $it.Name
    [System.IO.File]::WriteAllBytes($dcx, $buf)
    $raw = Undcx $buf
    $magic = [System.Text.Encoding]::ASCII.GetString($buf,0,4)
    if($raw){ [System.IO.File]::WriteAllBytes(($dcx -replace '\.dcx$',''), $raw); "$($it.Name) ok read=$got magic=$magic raw=$($raw.Length)" | Add-Content -Encoding ASCII $log }
    else { "$($it.Name) read=$got magic=$magic (not decompressed)" | Add-Content -Encoding ASCII $log }
  } catch { "$($it.Name) ERROR $($_.Exception.Message)" | Add-Content -Encoding ASCII $log }
}
# The tower mod's own arena event scripts, decompressed for reading
foreach($m in 'm45_00_00_00','m45_01_00_00','m45_02_00_00'){
  try {
    $p = Join-Path $here ("..\TestGame\ErdtreeTower-20261026-RandomAshPreview (2)\scene-assets\event\" + $m + '.emevd.dcx')
    $raw = Undcx ([System.IO.File]::ReadAllBytes($p))
    if($raw){ [System.IO.File]::WriteAllBytes((Join-Path $out ($m + '.emevd')), $raw); "$m.emevd ok raw=$($raw.Length)" | Add-Content -Encoding ASCII $log } else { "$m.emevd not decompressed" | Add-Content -Encoding ASCII $log }
  } catch { "$m.emevd ERROR $($_.Exception.Message)" | Add-Content -Encoding ASCII $log }
}
"done $(Get-Date -Format o)" | Add-Content -Encoding ASCII $log
