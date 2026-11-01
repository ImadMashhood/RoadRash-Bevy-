#!/usr/bin/env python3
"""Extract a Fedora user-local C++ toolchain, without sudo or RPM installation."""
import os,shlex,subprocess
from pathlib import Path
repo=Path(__file__).resolve().parents[2];cache=repo/'local/reference-tools';rpms=cache/'rpms';rpms.mkdir(parents=True,exist_ok=True)
root=Path.home()/'.local/share/roadrash-native/toolchain';root.mkdir(parents=True,exist_ok=True)
subprocess.run(['dnf','--disable-repo=*','--enable-repo=fedora','--enable-repo=updates','download',f'--destdir={rpms}','gcc-c++.x86_64','libstdc++-devel.x86_64','cmake.x86_64','cmake-data.noarch','SDL3-devel.x86_64','librhash.x86_64'],check=True)
for rpm in sorted(rpms.glob('*.rpm')):
    a=subprocess.Popen(['rpm2cpio',str(rpm)],stdout=subprocess.PIPE)
    b=subprocess.run(['cpio','-idm','--quiet'],stdin=a.stdout,cwd=root);a.stdout.close()
    if a.wait()!=0 or b.returncode!=0:raise SystemExit(f'Extraction failed: {rpm}')
# GCC's C support/runtime is already installed for Rust linking on this host.
version=subprocess.check_output(['gcc','-dumpversion'],text=True).strip().split('.')[0]
arch=subprocess.check_output(['gcc','-dumpmachine'],text=True).strip()
for directory in [f'usr/lib/gcc/{arch}/{version}',f'usr/libexec/gcc/{arch}/{version}','usr/lib64']:
    dest=root/directory;dest.mkdir(parents=True,exist_ok=True)
    for src in Path('/',directory).iterdir():
        p=dest/src.name
        if not p.exists() and not p.is_symlink():p.symlink_to(src)
q=shlex.quote(str(root))
(cache/'env.sh').write_text(f'''# Generated Fedora toolchain environment; source this file.
rr_tools_root={q}
export PATH="$rr_tools_root/usr/bin:$PATH"
export LD_LIBRARY_PATH="$rr_tools_root/usr/lib64${{LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}}"
export CPLUS_INCLUDE_PATH="$rr_tools_root/usr/include/c++/{version}:$rr_tools_root/usr/include/c++/{version}/{arch}:$rr_tools_root/usr/include/c++/{version}/backward${{CPLUS_INCLUDE_PATH:+:$CPLUS_INCLUDE_PATH}}"
export CXX="$rr_tools_root/usr/bin/g++"
export LIBRARY_PATH="$rr_tools_root/usr/lib/gcc/{arch}/{version}:$rr_tools_root/usr/lib64${{LIBRARY_PATH:+:$LIBRARY_PATH}}"
export LDFLAGS="-L$rr_tools_root/usr/lib/gcc/{arch}/{version} -L$rr_tools_root/usr/lib64${{LDFLAGS:+ $LDFLAGS}}"
export CMAKE_PREFIX_PATH="$rr_tools_root/usr${{CMAKE_PREFIX_PATH:+:$CMAKE_PREFIX_PATH}}"
''')
print('Ready:',cache/'env.sh')
