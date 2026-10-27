param([string]$GameDir = 'F:\SteamLibrary\steamapps\common\ELDEN RING\Game', [switch]$CheckOnly, [string]$PartyConfig)
$ErrorActionPreference = 'Stop'
function Set-TowerSpectatorMode([string]$Settings) {
    $pattern = '(?m)^[ \t]*always_spectate_on_death[ \t]*=[^\r\n]*'
    if ([regex]::IsMatch($Settings,$pattern)) {
        return [regex]::Replace($Settings,$pattern,'always_spectate_on_death = 1')
    }
    $section = [regex]'(?m)^\[GAMEPLAY\][ \t]*(\r?\n|$)'
    if (!$section.IsMatch($Settings)) {throw 'Missing GAMEPLAY section for spectator mode.'}
    $newline = if ($Settings.Contains("`r`n")) {"`r`n"} else {"`n"}
    return $section.Replace($Settings,('[GAMEPLAY]'+$newline+'always_spectate_on_death = 1'+$newline),1)
}
# Extra plugins (plugins/natives.toml, managed by the launcher)
# Each enabled [[natives]] section becomes an absolute path for ME3; disabled or missing plugins are skipped with a warning.
function Get-TowerPluginSections([string]$PackageRoot) {
    $file = Join-Path $PackageRoot 'plugins'
    $file = Join-Path $file 'natives.toml'
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) { return @() }
    $sections = [regex]::Matches([IO.File]::ReadAllText($file), '(?ms)^\s*\[\[natives\]\][^\r\n]*\r?\n.*?(?=^\s*\[\[|\z)')
    foreach ($section in $sections) {
        $text = $section.Value
        if ($text -match '(?m)^\s*enabled\s*=\s*false\b') { continue }
        $match = [regex]::Match($text, '(?m)^(\s*path\s*=\s*)[''"]([^''"\r\n]+)[''"]')
        if (-not $match.Success) { Write-Warning 'plugins/natives.toml: a [[natives]] entry has no single-line quoted path, skipped.'; continue }
        $path = $match.Groups[2].Value.Replace('\', '/').Replace('/', [IO.Path]::DirectorySeparatorChar)
        if (-not [IO.Path]::IsPathRooted($path)) { $path = Join-Path $PackageRoot $path }
        $path = [IO.Path]::GetFullPath($path)
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { Write-Warning "Plugin not found, skipped: $path"; continue }
        if ($path.Contains("'")) { Write-Warning "Plugin path contains a quote, skipped: $path"; continue }
        $text = $text.Substring(0, $match.Index) + $match.Groups[1].Value + "'" + $path.Replace('\', '/') + "'" + $text.Substring($match.Index + $match.Length)
        # enabled is the launcher's switch, not an ME3 option
        $text = [regex]::Replace($text, '(?m)^\s*enabled\s*=.*(\r?\n|\z)', '')
        $text.TrimEnd()
    }
}
# End extra plugins
$base = & (Join-Path $PSScriptRoot 'Launch-Preview.ps1') -GameDir $GameDir -CheckOnly
$root = $base.PackageDirectory
if ($PartyConfig) {
    $PartyConfig = (Resolve-Path -LiteralPath $PartyConfig -ErrorAction Stop).Path
    if ((Get-Item -LiteralPath $PartyConfig).Length -gt 8192) {throw 'Party configuration is too large.'}
    $party = [IO.File]::ReadAllText($PartyConfig) | ConvertFrom-Json
    if (!$party.session -or !$party.player -or !$party.host -or !$party.address) {throw 'Invalid party configuration.'}
}
$entry = Join-Path $root 'seamless-entry'
$launcher = $base.LoaderPath
$settingsPath = Join-Path $GameDir 'SeamlessCoop\ersc_settings.ini'
$ersc = Join-Path $GameDir 'SeamlessCoop\ersc.dll'
foreach ($file in @($launcher,$settingsPath,$ersc)) {
    if (!(Test-Path -LiteralPath $file -PathType Leaf)) {throw "Missing dependency: $file"}
}
if ((Get-FileHash -LiteralPath $ersc).Hash -ne 'FCD11A1849FD37851BB449BE30F1525CF498F63FD1FA568C4E15030EBE7CB251') {throw 'Expected inspected Seamless Co-op 2.0.1.'}
$proxy = Join-Path $GameDir 'dinput8.dll'
if (Test-Path -LiteralPath $proxy) {throw 'Existing dinput8.dll detected. It will not be overwritten.'}
$settings = [IO.File]::ReadAllText($settingsPath)
if ([regex]::Matches($settings,'(?m)^save_file_extension\s*=.*$').Count -ne 1) {throw 'Expected one save extension setting.'}
$saveExtension = if ($PartyConfig) {'ertowerparty'} else {'ertowercoop'}
$modified = [regex]::Replace($settings,'(?m)^save_file_extension\s*=.*$',('save_file_extension = '+$saveExtension))
$modified = [regex]::Replace($modified,'(?m)^allow_invaders\s*=.*$','allow_invaders = 0')
$modified = Set-TowerSpectatorMode $modified
if ($CheckOnly) {[pscustomobject]@{Entry=$launcher;SaveExtension=$saveExtension;Assets= (Join-Path $root 'scene-assets');MultiplayerVerified=$false};return}
if (Get-Process eldenring,ersc_launcher -ErrorAction SilentlyContinue) {throw 'Exit the game and Seamless launcher first.'}
$native = $base.DllPath
$assets = Join-Path $root 'scene-assets'
$runtime = Join-Path $entry $(if ($PartyConfig) {'party-runtime'} else {'runtime'})
New-Item -ItemType Directory -Path $runtime -Force | Out-Null
$assetLink = Join-Path $runtime 'scene-assets'
if (!(Test-Path -LiteralPath $assetLink)) {New-Item -ItemType Junction -Path $assetLink -Target $assets | Out-Null}
foreach ($path in @($native,$assets)) {if ($path.Contains("'")) {throw 'Unsupported quote in installation path.'}}
$config = @"
profileVersion = "v1"
start_online = false
[[supports]]
game = "eldenring"
[[natives]]
path = 'SeamlessCoop/ersc.dll'
[[natives]]
path = '$($native.Replace('\','/'))'
load_after = [{ id = "ersc.dll", optional = false }]
[[packages]]
path = '$($assets.Replace('\','/'))'
"@
$configPath = Join-Path $runtime 'seamless.me3'
# Custom DLL configuration
$editableProfile = $configPath
if (!(Test-Path -LiteralPath $editableProfile)) {
    [IO.File]::WriteAllText($editableProfile,$config,[Text.UTF8Encoding]::new($false))
}
$config = [IO.File]::ReadAllText($editableProfile)
$customPath = Join-Path $runtime 'custom-natives.toml'
if (!(Test-Path -LiteralPath $customPath)) {
    [IO.File]::WriteAllText($customPath, "# Extra [[natives]] entries; you can also edit the main .me3 profile directly.`r`n",[Text.UTF8Encoding]::new($false))
}
$nativeSections = '(?ms)^\s*\[\[natives\]\][^\r\n]*\r?\n.*?(?=^\s*\[\[|\z)'
function Get-NativeProfileKey([string]$section) {
    $match = [regex]::Match($section, '(?m)^\s*path\s*=\s*[''"]([^''"\r\n]+)[''"]\s*(?:#.*)?$')
    if (!$match.Success) { throw 'Extra native entry requires a single-line quoted path.' }
    $value = $match.Groups[1].Value.Replace('/', '\')
    if (![IO.Path]::IsPathRooted($value)) { $value = Join-Path $runtime $value }
    [IO.Path]::GetFullPath($value).ToUpperInvariant()
}
$knownNatives = @{}
foreach ($section in [regex]::Matches($config,$nativeSections)) {
    $knownNatives[(Get-NativeProfileKey $section.Value)] = $true
}
foreach ($section in [regex]::Matches([IO.File]::ReadAllText($customPath),$nativeSections)) {
    $key = Get-NativeProfileKey $section.Value
    if (!$knownNatives.ContainsKey($key)) {
        $config += "`r`n" + $section.Value
        $knownNatives[$key] = $true
    }
}
$configPath = Join-Path $runtime ([IO.Path]::GetFileNameWithoutExtension($editableProfile) + '.launch.me3')
# End custom DLL configuration
foreach ($section in @(Get-TowerPluginSections $root)) {
    $key = Get-NativeProfileKey $section
    if (!$knownNatives.ContainsKey($key)) {
        $config += "`r`n" + $section
        $knownNatives[$key] = $true
    }
}
# The editable profile keeps user entries; tower-owned paths always follow this installation.
$config = [regex]::Replace($config,"(?m)^(\s*path\s*=\s*)['""][^'""\r\n]*[\\/]erdtree_tower_\w+\.dll['""]",('${1}'''+$native.Replace('\','/').Replace('$','$$')+''''))
$config = [regex]::Replace($config,"(?m)^(\s*path\s*=\s*)['""][^'""\r\n]*[\\/]scene-assets['""]",('${1}'''+$assets.Replace('\','/').Replace('$','$$')+''''))
[IO.File]::WriteAllText($configPath,$config,(New-Object Text.UTF8Encoding($false)))
$private = Join-Path $runtime 'SeamlessCoop'
New-Item -ItemType Directory -Path $private -Force | Out-Null
Copy-Item -LiteralPath $ersc -Destination (Join-Path $private 'ersc.dll') -Force
foreach ($directory in @('crashpad','locale')) {
    $source = Join-Path (Split-Path $ersc) $directory
    if (Test-Path -LiteralPath $source -PathType Container) {Copy-Item -LiteralPath $source -Destination $private -Recurse -Force}
}
[IO.File]::WriteAllText((Join-Path $private 'ersc_settings.ini'),$modified,[Text.UTF8Encoding]::new($false))
$environment = @{
    ERDTREE_TOWER_PARTY=$PartyConfig
    ERDTREE_TOWER_SEAMLESS_ENTRY='1'; ERDTREE_TOWER_PREVIEW='1'; ERDTREE_TOWER_SEAMLESS_PROBE='0'
    ERDTREE_TOWER_LOG_DIR=$runtime
}
$previous = @{}
try {
    foreach ($key in $environment.Keys) {
        $previous[$key]=[Environment]::GetEnvironmentVariable($key)
        if ([string]::IsNullOrEmpty($environment[$key])) { Remove-Item -LiteralPath "Env:$key" -ErrorAction SilentlyContinue }
        else { [Environment]::SetEnvironmentVariable($key,$environment[$key]) }
    }
    Write-Host "ME3: Seamless + tower. Separate save: $saveExtension."
    & $launcher launch --exe (Join-Path $GameDir 'eldenring.exe') --profile $configPath --auto-detect --online false
    if ($LASTEXITCODE -ne 0) {throw "ME3 loader exit: $LASTEXITCODE"}
} finally {
    foreach ($key in $previous.Keys) {
        if ($null -eq $previous[$key]) { Remove-Item -LiteralPath "Env:$key" -ErrorAction SilentlyContinue }
        else { [Environment]::SetEnvironmentVariable($key,$previous[$key]) }
    }
}
