"""Upload one release's mirror folder to the Aliyun OSS mirror with ossutil, then check the line.

Usage: python distribution/upload_mirror.py dist/mirror-v<N> [--target oss://<bucket>/<prefix>] [--ossutil <path>]

The target defaults to the mirror URL in launcher/sources.json (https://<bucket>.oss-<region>.aliyuncs.com/<prefix>).
Credentials come from ossutil's own local config (run `ossutil config` once with a RAM user that may only write this
bucket); nothing secret is read or stored here. Order matters: data packs and the per-version manifest go up first,
releases/latest last, so a launcher checking mid-upload never sees a manifest whose pack is not there yet.
Re-running is safe: every file is simply overwritten with the same content.
"""
import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path
from urllib.parse import urlparse

ROOT = Path(__file__).resolve().parents[1]
SOURCES = ROOT / 'launcher' / 'sources.json'
sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_mirror  # noqa: E402


def mirror_url(sources=SOURCES):
    return json.loads(Path(sources).read_text(encoding='utf-8')).get('mirror', '').strip()


def target_from_url(url):
    """https://<bucket>.oss-<region>.aliyuncs.com/<prefix> -> oss://<bucket>/<prefix>"""
    parsed = urlparse(url)
    match = re.fullmatch(r'([a-z0-9][a-z0-9-]{1,61}[a-z0-9])\.oss-[a-z0-9-]+\.aliyuncs\.com', parsed.hostname or '')
    if parsed.scheme != 'https' or not match:
        raise SystemExit(f'Cannot derive an oss:// target from {url!r}; pass --target oss://<bucket>/<prefix>')
    return f'oss://{match.group(1)}/{parsed.path.strip("/")}'.rstrip('/')


def plan(mirror_dir, target):
    """(local file, oss key) pairs in upload order."""
    releases = Path(mirror_dir) / 'releases'
    if not (releases / 'latest' / 'download' / check_mirror.MANIFEST).is_file():
        raise SystemExit(f'{releases} is not a mirror folder written by package.py')
    def key(path):
        return f'{target}/{path.relative_to(releases).as_posix()}'
    downloads = sorted((p for p in (releases / 'download').rglob('*') if p.is_file()), key=lambda p: (p.name == check_mirror.MANIFEST, p.as_posix()))
    latest = sorted(p for p in (releases / 'latest').rglob('*') if p.is_file())
    return [(p, key(p)) for p in downloads + latest]


def find_ossutil(explicit=None):
    tool = explicit or shutil.which('ossutil') or shutil.which('ossutil64')
    if not tool:
        raise SystemExit('ossutil not found: install it from https://help.aliyun.com/zh/oss/developer-reference/ossutil '
                         'and run `ossutil config`, or pass --ossutil <path>')
    return tool


def upload(mirror_dir, target, tool, run=subprocess.run):
    for local, key in plan(mirror_dir, target):
        print(f'upload {local.name} -> {key}')
        run([tool, 'cp', str(local), key, '-f'], check=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description='Upload a mirror folder to Aliyun OSS and check it')
    parser.add_argument('mirror_dir', type=Path, help='dist/mirror-v<N> written by package.py')
    parser.add_argument('--target', help='oss://<bucket>/<prefix>; default: derived from launcher/sources.json')
    parser.add_argument('--ossutil', help='path to ossutil / ossutil64')
    args = parser.parse_args(argv)
    url = mirror_url()
    target = (args.target or target_from_url(url)).rstrip('/')
    version = json.loads((args.mirror_dir / 'releases/latest/download' / check_mirror.MANIFEST).read_text(encoding='utf-8'))['version']
    upload(args.mirror_dir, target, find_ossutil(args.ossutil))
    if not url:
        print('launcher/sources.json has no mirror URL, skipped the download check')
        return 0
    _, problems = check_mirror.check(url, version)
    for problem in problems:
        print('问题：' + problem)
    if problems:
        return 1
    print(f'镜像 v{version} 上传完成并检查通过：{url}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
