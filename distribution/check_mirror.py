"""Check that a download line (the Aliyun OSS mirror, or GitHub) serves the launcher's update data correctly.

Usage: python distribution/check_mirror.py <base URL> [--expect <version>]
  e.g. python distribution/check_mirror.py https://eldenring-mods-hz.oss-cn-hangzhou.aliyuncs.com/releases --expect 96

Reads <base>/latest/download/tower-release.json, then for every data pack the manifest references fetches its smallest
blob with an HTTP Range request (the launcher only ever downloads ranges) and checks the 206 answer and the SHA-256.
"""
import argparse
import hashlib
import json
import sys
import urllib.error
import urllib.request
import zlib
from urllib.parse import quote

MANIFEST = 'tower-release.json'


def fetch(url, first=None, last=None):
    request = urllib.request.Request(url, headers={'User-Agent': 'ErdtreeTowerMirrorCheck'})
    if first is not None:
        request.add_header('Range', f'bytes={first}-{last}')
    with urllib.request.urlopen(request, timeout=60) as response:
        return response.status, response.read()


def check(base, expect=None):
    """Returns (manifest or None, list of problems in Chinese)."""
    base = base.rstrip('/')
    try:
        _, body = fetch(f'{base}/latest/download/{MANIFEST}')
        manifest = json.loads(body.decode('utf-8'))
    except urllib.error.HTTPError as error:
        error.close()
        return None, [f'读不到最新清单 latest/download/{MANIFEST}：HTTP {error.code}（文件没传、路径不对，或存储桶不是公有读）']
    except (urllib.error.URLError, OSError, ValueError) as error:
        return None, [f'读不到最新清单：{error}']
    problems = []
    if expect is not None and int(manifest.get('version', 0)) != expect:
        problems.append(f'最新清单是 v{manifest.get("version")}，不是 v{expect}（latest 目录没更新？）')
    # One sample per pack: the smallest non-empty blob
    samples = {}
    for sha, blob in manifest.get('blobs', {}).items():
        if blob['length'] <= 0:
            continue
        key = (blob['tag'], blob['asset'])
        if key not in samples or blob['length'] < samples[key][1]['length']:
            samples[key] = (sha, blob)
    for (tag, asset), (sha, blob) in sorted(samples.items()):
        name = f'download/{tag}/{asset}'
        try:
            status, data = fetch(f'{base}/download/{quote(tag)}/{quote(asset)}', blob['offset'], blob['offset'] + blob['length'] - 1)
        except urllib.error.HTTPError as error:
            error.close()
            problems.append(f'{name}：HTTP {error.code}（这个版本的数据包没传？）')
            continue
        except (urllib.error.URLError, OSError) as error:
            problems.append(f'{name}：{error}')
            continue
        if status != 206:
            problems.append(f'{name}：服务器回了 {status}，不支持分段下载')
            continue
        content = zlib.decompress(data, -15) if blob['method'] == 'deflate' else data
        if len(content) != blob['size'] or hashlib.sha256(content).hexdigest() != sha:
            problems.append(f'{name}：数据和清单对不上（传错了文件？）')
    return manifest, problems


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Check a launcher download line')
    parser.add_argument('base', help='line base URL, ending in /releases')
    parser.add_argument('--expect', type=int, help='version the latest manifest should have')
    args = parser.parse_args()
    manifest, problems = check(args.base, args.expect)
    if manifest:
        packs = sorted({(b['tag'], b['asset']) for b in manifest['blobs'].values()})
        print(f'最新清单 v{manifest["version"]}：{len(manifest["files"])} 个文件，引用 {len(packs)} 个数据包')
    for problem in problems:
        print('问题：' + problem)
    if problems:
        sys.exit(1)
    print('线路正常：清单可读，每个数据包都能分段下载且校验通过')
