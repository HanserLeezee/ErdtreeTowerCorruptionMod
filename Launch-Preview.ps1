param([string]$GameDir = 'F:\SteamLibrary\steamapps\common\ELDEN RING\Game', [switch]$CheckOnly, [ValidateSet('ME3','YAFSML')][string]$Loader='ME3')
$ErrorActionPreference = 'Stop'
$loaderSupport=Join-Path $PSScriptRoot 'Tower-Loader.ps1'
if(!(Test-Path -LiteralPath $loaderSupport)){$loaderSupport=Join-Path $PSScriptRoot 'distribution/Tower-Loader.ps1'}
. $loaderSupport
$exePath = Join-Path $GameDir 'eldenring.exe'
if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) { throw '找不到游戏，请传入 -GameDir' }
if ((Get-Item -LiteralPath $exePath).VersionInfo.FileVersion -ne '2.7.2.0') { throw '验证包仅支持游戏文件版本 2.7.2.0' }
$sha256 = [System.Security.Cryptography.SHA256]::Create()
$exeStream = [System.IO.File]::OpenRead($exePath)
try { $exeHash = [System.BitConverter]::ToString($sha256.ComputeHash($exeStream)).Replace('-', '').ToLowerInvariant() }
finally { $exeStream.Dispose(); $sha256.Dispose() }
if ($exeHash -ne 'f9118fea2cfd8fa3901e8060bfadb1552cb2471771ded8a5b662c4e298ffb328') {
    throw '游戏文件与已核实的版本不匹配'
}
function Get-NativeDllName([string]$Root) {
    $profilePath = Join-Path $Root 'preview.me3'
    if (-not (Test-Path -LiteralPath $profilePath -PathType Leaf)) { return $null }
    $names = @([regex]::Matches([System.IO.File]::ReadAllText($profilePath), "path\s*=\s*['""](erdtree_tower_preview_v\d+\.dll)['""]") | ForEach-Object { $_.Groups[1].Value })
    if ($names.Count -ne 1) { return $null }
    return $names[0]
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
$packageRoot = $null
$dllName = $null
$candidates = @($PSScriptRoot, (Join-Path $PSScriptRoot 'release-preview'))
foreach ($candidate in $candidates) {
    if (-not (Test-Path -LiteralPath (Join-Path $candidate 'scene-assets\reward-art\rare-blue.rgba'))) { continue }
    if (-not (Test-Path -LiteralPath (Join-Path $candidate 'scene-assets\reward-art\rarity-cards.rgba'))) { continue }
    $candidateDll = Get-NativeDllName $candidate
    if (-not $candidateDll) { continue }
    $requiredFiles = @('loader\bin\me3.exe', $candidateDll, 'preview.me3', 'scene-assets\regulation.bin', 'scene-assets\msg\zhocn\menu_dlc02.msgbnd.dcx', 'scene-assets\msg\zhocn\item_dlc02.msgbnd.dcx', 'scene-assets\script\talk\m00_00_00_00.talkesdbnd.dcx', 'scene-assets\event\m11_10_00_00.emevd.dcx', 'scene-assets\enemy-pools.txt', 'scene-assets\oath-art\cards.rgba', 'scene-assets\oath-art\background.rgba', 'scene-assets\oath-art\button.rgba')
    $requiredFiles += 'scene-assets\map\mapstudio\m11_10_00_00.msb.dcx'
    foreach ($map in @('m45_00_00_00', 'm45_01_00_00', 'm45_02_00_00')) {
        $requiredFiles += "scene-assets\map\mapstudio\$map.msb.dcx"
        $requiredFiles += "scene-assets\event\$map.emevd.dcx"
        $requiredFiles += "scene-assets\script\talk\$map.talkesdbnd.dcx"
    }
    foreach ($map in @('m10_00_00_00', 'm12_01_00_00', 'm11_00_00_00')) {
        $requiredFiles += "scene-assets\map\mapstudio\$map.msb.dcx"
        $requiredFiles += "scene-assets\event\$map.emevd.dcx"
    }
    $requiredFiles += 'scene-assets\battlefields.txt'
    $requiredFiles += 'scene-assets\reward-details.tsv'
    $requiredFiles += 'scene-assets\reward-art\card.rgba'
    $requiredFiles += @('scene-assets\map-art\paper.rgba', 'scene-assets\map-art\icons.rgba')
    $detailsFile = Join-Path $candidate 'scene-assets\reward-details.tsv'
    $generatedIconIds = @{}
    $modeFile = Join-Path $candidate 'asset-mode.txt'
    $assetManifest = Join-Path $candidate 'asset-patches\manifest.json'
    if ((Test-Path -LiteralPath $modeFile) -and [IO.File]::ReadAllText($modeFile).Trim() -eq 'local-v1' -and (Test-Path -LiteralPath $assetManifest)) {
        $iconRecipes = ([IO.File]::ReadAllText($assetManifest) | ConvertFrom-Json).Icons
        foreach ($iconRecipe in $iconRecipes) { $generatedIconIds[[string]$iconRecipe.Id] = $true }
        if ($generatedIconIds.Count -gt 0) { $requiredFiles += @('Tower.AssetRestore.exe', 'Tower.IconCodec.dll') }
    }
    if (Test-Path -LiteralPath $detailsFile -PathType Leaf) {
        foreach ($line in Get-Content -LiteralPath $detailsFile -Encoding UTF8) {
            $fields = $line -split "`t"
            if ($fields.Count -eq 7 -and -not $generatedIconIds.ContainsKey([string]$fields[1])) { $requiredFiles += "scene-assets\reward-icons\$($fields[1]).rgba" }
        }
    }
    $missingFiles = @($requiredFiles | Where-Object { -not (Test-Path -LiteralPath (Join-Path $candidate $_) -PathType Leaf) })
    if ($missingFiles.Count -eq 0) { $packageRoot = $candidate; $dllName = $candidateDll; break }
}
if (-not $packageRoot) { throw "找不到完整验证包，已检查目录：$($candidates -join '；')。请完整解压验证包。" }
$loaderPath = Get-TowerLoaderPath $packageRoot $Loader
$dll = Join-Path $packageRoot $dllName
$profile = Join-Path $packageRoot 'preview.me3'
if ($CheckOnly) {
    [pscustomobject]@{ PackageDirectory = $packageRoot; LoaderPath = $loaderPath; DllPath = $dll; ProfilePath = $profile; Loader = $Loader }
    return
}
if (Get-Process -Name eldenring -ErrorAction SilentlyContinue) { throw '请先退出正在运行的艾尔登法环' }
$assetMode = Join-Path $packageRoot 'asset-mode.txt'
if ((Test-Path -LiteralPath $assetMode) -and [IO.File]::ReadAllText($assetMode).Trim() -notin @('full', 'local-v1')) { throw '资源包模式损坏，请校验并修复文件。' }
if (-not (Test-Path -LiteralPath $assetMode) -and (Test-Path -LiteralPath (Join-Path $packageRoot 'asset-patches\manifest.json'))) { throw '缺少资源包模式，请校验并修复文件。' }
if ((Test-Path -LiteralPath $assetMode) -and [IO.File]::ReadAllText($assetMode).Trim() -eq 'local-v1') {
    if (-not (Test-Path -LiteralPath (Join-Path $packageRoot 'asset-patches\manifest.json'))) { throw '缺少资源补丁清单，请校验并修复文件。' }
    $restore = Join-Path $packageRoot 'Tower.AssetRestore.exe'
    if (-not (Test-Path -LiteralPath $restore)) { throw '缺少本地资源生成器，请校验并修复文件。' }
    & $restore $packageRoot $GameDir
    if ($LASTEXITCODE -ne 0) { throw '本地资源生成失败，请查看日志或使用完整整合包。' }
}
$previousMarker = $env:ERDTREE_TOWER_PREVIEW
$previousLogDir = $env:ERDTREE_TOWER_LOG_DIR
try {
    $env:ERDTREE_TOWER_PREVIEW = '1'
    $env:ERDTREE_TOWER_LOG_DIR = $packageRoot
    $logPath = Join-Path $packageRoot 'tower-preview.log'
    [System.IO.File]::WriteAllText($logPath, "Tower preview launch: $(Get-Date -Format o)`r`n")
    Write-Host "预览初始化日志：$logPath"
    $plugins = @(Get-TowerPluginSections $packageRoot)
    if ($plugins.Count -gt 0) {
        # The shipped preview.me3 stays untouched (updates replace it); plugins go into a generated launch profile next to it
        $launchProfile = Join-Path $packageRoot 'preview.launch.me3'
        $text = [System.IO.File]::ReadAllText($profile).TrimEnd() + "`r`n`r`n" + ($plugins -join "`r`n`r`n") + "`r`n"
        [System.IO.File]::WriteAllText($launchProfile, $text, (New-Object System.Text.UTF8Encoding($false)))
        $profile = $launchProfile
        Write-Host "额外 DLL 插件：$($plugins.Count) 个"
    }
    Invoke-TowerLoader $packageRoot $Loader $exePath $profile
} finally {
    $env:ERDTREE_TOWER_PREVIEW = $previousMarker
    $env:ERDTREE_TOWER_LOG_DIR = $previousLogDir
}
