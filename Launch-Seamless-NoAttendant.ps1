param([string]$GameDir='F:\SteamLibrary\steamapps\common\ELDEN RING\Game',[switch]$CheckOnly,[string]$PartyConfig)
$ErrorActionPreference='Stop'
$previous=[Environment]::GetEnvironmentVariable('ERDTREE_TOWER_NO_ATTENDANT')
try {
    $env:ERDTREE_TOWER_NO_ATTENDANT='1'
    & (Join-Path $PSScriptRoot 'Launch-Seamless-Tower.ps1') -GameDir $GameDir -CheckOnly:$CheckOnly -PartyConfig $PartyConfig
}finally{
    [Environment]::SetEnvironmentVariable('ERDTREE_TOWER_NO_ATTENDANT',$previous)
}
