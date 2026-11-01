#!/usr/bin/env python3
"""Check every inventoried codepoint against the actual embedded font via FreeType."""
import argparse
import ctypes as C
import ctypes.util
import hashlib
import json
from pathlib import Path


def audit(font, inventory):
    ft = C.CDLL(ctypes.util.find_library('freetype'))
    library, face = C.c_void_p(), C.c_void_p()
    ft.FT_Init_FreeType.argtypes = [C.POINTER(C.c_void_p)]
    ft.FT_New_Face.argtypes = [C.c_void_p, C.c_char_p, C.c_long, C.POINTER(C.c_void_p)]
    ft.FT_Get_Char_Index.argtypes = [C.c_void_p, C.c_ulong]
    ft.FT_Get_Char_Index.restype = C.c_uint
    ft.FT_Done_Face.argtypes = [C.c_void_p]
    ft.FT_Done_FreeType.argtypes = [C.c_void_p]
    assert ft.FT_Init_FreeType(C.byref(library)) == 0
    try:
        assert ft.FT_New_Face(library, str(font.resolve()).encode(), 0, C.byref(face)) == 0
        codepoints = json.loads(inventory.read_text())['printable_codepoints']
        missing = [c for c in codepoints if ft.FT_Get_Char_Index(face, c) == 0]
        return dict(outcome='passed' if not missing else 'failed', printable_characters=len(codepoints),
                    missing_codepoints=missing, font_sha256=hashlib.sha256(font.read_bytes()).hexdigest())
    finally:
        if face: ft.FT_Done_Face(face)
        ft.FT_Done_FreeType(library)


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('font', type=Path)
    p.add_argument('inventory', type=Path)
    a = p.parse_args()
    result = audit(a.font, a.inventory)
    print(json.dumps(result, indent=2))
    raise SystemExit(0 if result['outcome'] == 'passed' else 1)
