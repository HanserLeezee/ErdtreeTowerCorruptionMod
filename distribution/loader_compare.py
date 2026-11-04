"""Prepare isolated ME3/YAFSML 0.11.0 test entrances; never replace the installed launcher."""
from pathlib import Path
import argparse
import hashlib
import json
import re
import shutil
import subprocess

YAF_HASHES = {
    'YAFSML.exe': 'a7f1447c8db43905c3eaf8653f5306dbb68a7418aff0606335f7f0d6de7b5437',
    'YAFSML.dll': '1a3ea05b0d3122bca170ae80f8fcec10659539939bc85b89bee4a65dfb7debe4',
}


def clean(path):
    text = str(path).replace('\\', '/')
    if any(c in text for c in "\r\n|';\"="):
        raise ValueError('Unsupported configuration path: ' + text)
    return text


def configs(native, assets, seamless):
    native, assets = clean(native), clean(assets)
    me3 = 'profileVersion = "v1"\nstart_online = false\nsavefile = "ER_TOWER_LOADER_TEST.sl2"\n\n[[supports]]\ngame = "eldenring"\n'
    dlls = ''
    if seamless:
        seamless = clean(seamless)
        me3 += f"\n[[natives]]\npath = '{seamless}'\n"
        dlls += f'seamless={seamless}\n'
    me3 += f"\n[[natives]]\npath = '{native}'\n"
    if seamless: me3 += 'load_after = [{ id = "ersc.dll", optional = false }]\n'
    me3 += f"\n[[packages]]\npath = '{assets}'\n"
    dlls += f'tower={native}' + ('|after,seamless' if seamless else '') + '\n'
    yaf = ('game=eldenring\n\n[patch]\nskip_intro=1\nprevent_regulation_save_write=1\n'
           'patch_mem=1\npatch_mem_dedicated_heap=1\npatch_mem_heap_size=0\nboot_boost=1\ndisable_arxan=0\n'
           'replace_save_filename=ER_TOWER_LOADER_TEST.sl2\nreplace_seamless_coop_save_filename=\nenable_ime=0\n'
           '\n[tweak]\ncpu_affinity=0\n\n[log]\nconsole=0\nlog_file=1\nlog_level=info\n'
           f'\n[dll]\n{dlls}\n[mod]\ntower={assets}\n')
    return me3, yaf


def ps_literal(value):
    return "'" + str(value).replace("'", "''") + "'"


def prepare(package, release, game, output, mode):
    package, release, game, output = (Path(p).resolve() for p in (package, release, game, output))
    if output.exists(): raise ValueError('Choose a new comparison directory')
    for name, expected in YAF_HASHES.items():
        if hashlib.sha256((release / name).read_bytes()).hexdigest() != expected:
            raise ValueError('Expected verified YAFSML 0.11.0: ' + name)
    profile = (package / 'preview.me3').read_text(encoding='utf-8-sig')
    names = re.findall(r"path\s*=\s*['\"](erdtree_tower_preview_v\d+\.dll)['\"]", profile)
    if len(names) != 1: raise ValueError('Expected one tower DLL')
    native = package / names[0]
    assets = package / 'scene-assets'
    for path in (native, assets / 'regulation.bin', assets / 'menu/hi/01_common.tpf.dcx', game / 'eldenring.exe', package / 'loader/bin/me3.exe'):
        if not path.is_file(): raise ValueError('Prepare installed resources first: ' + str(path))
    plugin_config = package / 'plugins/natives.toml'
    if plugin_config.exists() and '[[natives]]' in plugin_config.read_text(encoding='utf-8-sig'):
        raise ValueError('Extra plugins need explicit mapping before this controlled comparison')
    source_coop = package / 'seamless-entry/runtime/SeamlessCoop'
    if mode == 'Seamless' and not (source_coop / 'ersc_settings.ini').exists():
        raise ValueError('Run the standard Seamless entry once to prepare its private settings')
    output.mkdir(parents=True)
    summary = {'mode': mode, 'yafsml': '0.11.0', 'native': str(native),
               'nativeSha256': hashlib.sha256(native.read_bytes()).hexdigest(),
               'assets': str(assets), 'save': 'ER_TOWER_LOADER_TEST.sl2', 'coopExtension': 'ertowerloadertest'}
    for loader in ('ME3', 'YAFSML'):
        runtime = output / loader; runtime.mkdir()
        subprocess.run(['cmd', '/d', '/c', 'mklink', '/J', str(runtime / 'scene-assets'), str(assets)], check=True, capture_output=True)
        ersc = None
        if mode == 'Seamless':
            private = runtime / 'SeamlessCoop'; shutil.copytree(source_coop, private)
            settings_path = private / 'ersc_settings.ini'
            settings = settings_path.read_text(encoding='utf-8-sig')
            if len(re.findall(r'(?m)^save_file_extension\s*=', settings)) != 1: raise ValueError('Invalid Seamless save setting')
            settings = re.sub(r'(?m)^save_file_extension\s*=.*$', 'save_file_extension = ertowerloadertest', settings)
            settings_path.write_text(settings, encoding='utf-8')
            ersc = private / 'ersc.dll'
        me3, yaf = configs(native, assets, ersc)
        if loader == 'YAFSML':
            for name in ('YAFSML.exe', 'YAFSML.dll', 'LICENSE', 'THIRD_PARTY_NOTICES.md'):
                shutil.copy2(release / name, runtime / name)
            config = runtime / 'YAFSML.ini'; config.write_text(yaf, encoding='utf-8')
            exe = runtime / 'YAFSML.exe'
            arguments = ['--launch-target', 'eldenring', '--game-path', str(game / 'eldenring.exe'), '--config', str(config)]
        else:
            config = runtime / 'comparison.me3'; config.write_text(me3, encoding='utf-8')
            exe = package / 'loader/bin/me3.exe'
            arguments = ['launch', '--exe', str(game / 'eldenring.exe'), '--profile', str(config), '--auto-detect', '--online', 'false', '--skip-steam-init', 'true']
        # Start-Process joins ArgumentList; quote each path for the Windows command line.
        arguments_text = subprocess.list2cmdline(arguments)
        script = f'''param([switch]$CheckOnly)
$ErrorActionPreference = 'Stop'
if ($CheckOnly) {{ Write-Output {ps_literal(loader + ' ' + mode + ' comparison ready')}; return }}
if (Get-Process eldenring -ErrorAction SilentlyContinue) {{ throw 'Save and exit the running game first.' }}
$env:ERDTREE_TOWER_PREVIEW = '1'
$env:ERDTREE_TOWER_LOG_DIR = $PSScriptRoot
$env:ERDTREE_TOWER_SEAMLESS_ENTRY = {ps_literal('1' if mode == 'Seamless' else '')}
Remove-Item Env:ERDTREE_TOWER_PARTY -ErrorAction SilentlyContinue
Remove-Item Env:ERDTREE_TOWER_SEAMLESS_PROBE -ErrorAction SilentlyContinue
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss-fff'
$launch = @{{loader={ps_literal(loader)};mode={ps_literal(mode)};startedUtc=[DateTime]::UtcNow.ToString('o')}}
$launch | ConvertTo-Json | Set-Content (Join-Path $PSScriptRoot "launch-$stamp.json") -Encoding UTF8
$process = Start-Process -FilePath {ps_literal(exe)} -ArgumentList {ps_literal(arguments_text)} -WorkingDirectory $PSScriptRoot -WindowStyle Hidden -RedirectStandardOutput (Join-Path $PSScriptRoot "stdout-$stamp.log") -RedirectStandardError (Join-Path $PSScriptRoot "stderr-$stamp.log") -PassThru
Write-Output "{loader} started; launcher PID=$($process.Id)"
'''
        (runtime / 'Start-Test.ps1').write_text(script, encoding='utf-8-sig')
        (runtime / 'Start-Test.cmd').write_text('@echo off\n"%SystemRoot%\\System32\\WindowsPowerShell\\v1.0\\powershell.exe" -NoProfile -ExecutionPolicy Bypass -File "%~dp0Start-Test.ps1"\nif errorlevel 1 pause\n', encoding='ascii')
    (output / 'comparison.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
    print(json.dumps(summary, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('package', 'release', 'game', 'output'): parser.add_argument(name, type=Path)
    parser.add_argument('--mode', choices=('Solo', 'Seamless'), default='Solo')
    args = parser.parse_args()
    prepare(args.package, args.release, args.game, args.output, args.mode)
