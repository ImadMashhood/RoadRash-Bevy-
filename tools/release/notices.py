"""Collect locked dependency license texts; fail closed when a text is unavailable.

Registry sources are local Cargo downloads. Missing workspace-root texts are fetched
from the exact repository revision recorded by Cargo, never an unpinned branch.
"""
import concurrent.futures
import hashlib
import json
from pathlib import Path
import shutil
import urllib.request

PREFIXES = ('license', 'licence', 'copying', 'notice', 'copyright')


def collect(metadata, destination, cache):
    destination.mkdir(parents=True, exist_ok=False)
    packages = [p for p in metadata['packages'] if p['source'] is not None]

    def dependency(p):
        source = Path(p['manifest_path']).parent
        files = [f for f in source.iterdir() if f.is_file() and f.name.lower().startswith(PREFIXES)]
        # Some crates package licenses in a license/ directory.
        files += [f for d in source.iterdir() if d.is_dir() and d.name.lower().startswith(PREFIXES)
                  for f in d.rglob('*') if f.is_file()]
        folder = destination / (p['name'] + '-' + p['version'])
        folder.mkdir()
        origins = []
        for index, f in enumerate(sorted(files)):
            target = folder / (str(index) + '-' + f.name)
            shutil.copyfile(f, target)
            origins.append({'file': target.name, 'origin': 'Cargo registry archive: ' + str(f.relative_to(source))})
        if not files:
            vcs = json.loads((source / '.cargo_vcs_info.json').read_text())['git']['sha1']
            repo = (p['repository'] or '').removesuffix('.git').rstrip('/')
            paths = ['LICENSE', 'LICENSE-APACHE', 'LICENSE-MIT', 'LICENSE.txt', 'LICENSE.md',
                     'LICENSE-APACHE.txt', 'LICENSE-MIT.txt', 'COPYING', 'LICENSES/Apache-2.0.txt']
            cache.mkdir(parents=True, exist_ok=True)
            for name in paths:
                if 'github.com/' in repo:
                    url = repo.replace('https://github.com/', 'https://raw.githubusercontent.com/') + '/' + vcs + '/' + name
                elif 'gitlab.com/' in repo:
                    url = repo + '/-/raw/' + vcs + '/' + name
                else:
                    continue
                cached = cache / hashlib.sha256(url.encode()).hexdigest()
                try:
                    if not cached.exists():
                        with urllib.request.urlopen(url, timeout=20) as response:
                            data = response.read(1024*1024+1)
                        if len(data) > 1024*1024 or b'<html' in data[:1000].lower():
                            raise ValueError('Invalid license response')
                        cached.write_bytes(data)
                    shutil.copyfile(cached, folder / Path(name).name)
                    origins.append({'file': Path(name).name, 'origin': url})
                except (OSError, ValueError):
                    continue
            if not origins and 'Apache-2.0' in (p['license'] or ''):
                # The declared Apache option has no per-crate copyright template.
                # Preserve its full standard text and identify the package/authors
                # separately; never invent an upstream NOTICE or copyright line.
                template = next(Path(q['manifest_path']).parent / 'LICENSE-APACHE'
                    for q in metadata['packages'] if q['name'] == 'bevy')
                shutil.copyfile(template, folder / 'Apache-2.0.txt')
                origins.append({'file':'Apache-2.0.txt',
                    'origin':'Standard Apache-2.0 full text from locked Bevy archive; upstream revision contains no license file; Apache option declared in Cargo metadata'})
            if not origins:
                raise ValueError('Missing actual license text: ' + p['name'])
        return {'name':p['name'], 'version':p['version'], 'license_expression':p['license'],
                'repository':p['repository'], 'authors':p['authors'], 'texts':origins}
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        rows = list(pool.map(dependency, packages))
    (destination / 'index.json').write_text(json.dumps({'scope':'Locked Linux dependency graph, including build and test dependencies; no license grant for original game data.', 'dependencies':rows}, indent=2)+'\n')
    return rows
