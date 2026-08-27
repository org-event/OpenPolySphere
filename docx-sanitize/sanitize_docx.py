#!/usr/bin/env python3
"""
Full DOCX sanitizer: replace sensitive content while keeping the template.

- Text (body, headers/footers, footnotes, comments, text boxes, charts…):
  each word/number → random chars of the same length (script/case preserved).
- Raster images (PNG/JPEG/…): same-pixel-size gray placeholders.
- Vector images (EMF/WMF/SVG): same-dimension blank placeholders.
- Embeddings / OLE binaries: wiped (zero-filled, same length).
- Document properties, thumbnail, hyperlinks, alt-text: sanitized.

Usage:
  python3 sanitize_docx.py input.docx
  python3 sanitize_docx.py input.docx -o output.docx
  python3 sanitize_docx.py input.docx --seed 42
"""

from __future__ import annotations

import argparse
import io
import random
import re
import string
import struct
import sys
import zipfile
from pathlib import Path
from xml.etree import ElementTree as ET

try:
    from PIL import Image, ImageDraw
except ImportError:
    print("Install Pillow: pip install Pillow", file=sys.stderr)
    sys.exit(1)

# --- namespaces (register so re-serialized XML stays Word-friendly) ---
NS = {
    "w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main",
    "a": "http://schemas.openxmlformats.org/drawingml/2006/main",
    "r": "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    "v": "urn:schemas-microsoft-com:vml",
    "wp": "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing",
    "pic": "http://schemas.openxmlformats.org/drawingml/2006/picture",
    "c": "http://schemas.openxmlformats.org/drawingml/2006/chart",
    "cp": "http://schemas.openxmlformats.org/package/2006/metadata/core-properties",
    "dc": "http://purl.org/dc/elements/1.1/",
    "dcterms": "http://purl.org/dc/terms/",
    "ep": "http://schemas.openxmlformats.org/officeDocument/2006/extended-properties",
    "cus": "http://schemas.openxmlformats.org/officeDocument/2006/custom-properties",
    "rel": "http://schemas.openxmlformats.org/package/2006/relationships",
    "mc": "http://schemas.openxmlformats.org/markup-compatibility/2006",
    "wps": "http://schemas.microsoft.com/office/word/2010/wordprocessingShape",
    "wpg": "http://schemas.microsoft.com/office/word/2010/wordprocessingGroup",
}
for prefix, uri in NS.items():
    ET.register_namespace(prefix, uri)
# default empty prefix sometimes used — leave alone

RASTER_EXTS = {".png", ".jpg", ".jpeg", ".gif", ".bmp", ".tif", ".tiff", ".webp"}
VECTOR_EXTS = {".emf", ".wmf", ".emz", ".wmz", ".svg"}
# binaries we wipe (same length) — content is opaque / often sensitive
WIPE_DIR_PREFIXES = (
    "word/media/",
    "word/embeddings/",
    "word/diagrams/",
    "word/charts/_rels/",  # handled as xml separately when .xml
)

LATIN_LOWER = string.ascii_lowercase
LATIN_UPPER = string.ascii_uppercase
CYR_LOWER = "абвгдеёжзийклмнопрстуфхцчшщъыьэюя"
CYR_UPPER = "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ"
DIGITS = string.digits

WORD_RE = re.compile(
    r"[^\W_]+(?:['’ʼ][^\W_]+)*|[0-9]+(?:[.,][0-9]+)*",
    re.UNICODE,
)

# XML text element local-names that hold user-visible / extractable text
TEXT_LOCAL_NAMES = {
    "t",  # w:t, a:t
    "instrText",
    "delText",
    "caption",
    "v",  # chart cached values c:v
    "f",  # chart formulas sometimes
}

# Attributes that often carry alt-text / titles / tooltips
SENSITIVE_ATTRS = {
    "descr",
    "title",
    "name",  # shape names can leak; scramble if looks like words
    "alt",
    "tooltip",
    "id",  # only when long text? skip short ids — handled below
}

META_TEXT_LOCAL = {
    "title",
    "subject",
    "creator",
    "description",
    "keywords",
    "category",
    "contentStatus",
    "version",
    "revision",
    "lastModifiedBy",
    "Company",
    "Manager",
    "HyperlinkBase",
    "Template",
    "Application",
    "presentationFormat",
    "company",
    "manager",
}


def _local(tag: str) -> str:
    return tag.rsplit("}", 1)[-1] if "}" in tag else tag


def _rand_char_like(ch: str, rng: random.Random) -> str:
    if ch.isdigit():
        return rng.choice(DIGITS)
    if ch in LATIN_LOWER or "a" <= ch <= "z":
        return rng.choice(LATIN_LOWER)
    if ch in LATIN_UPPER or "A" <= ch <= "Z":
        return rng.choice(LATIN_UPPER)
    if ch in CYR_LOWER:
        return rng.choice(CYR_LOWER)
    if ch in CYR_UPPER:
        return rng.choice(CYR_UPPER)
    if ch.isalpha():
        return rng.choice(LATIN_UPPER if ch.isupper() else LATIN_LOWER)
    if ch.isalnum():
        return rng.choice(DIGITS)
    return ch


def scramble_word(word: str, rng: random.Random) -> str:
    return "".join(_rand_char_like(c, rng) for c in word)


def scramble_text(text: str, rng: random.Random) -> str:
    if not text or not text.strip():
        return text
    out: list[str] = []
    last = 0
    for m in WORD_RE.finditer(text):
        out.append(text[last : m.start()])
        out.append(scramble_word(m.group(0), rng))
        last = m.end()
    out.append(text[last:])
    return "".join(out)


def scramble_url(url: str, rng: random.Random) -> str:
    """Keep scheme and separators; scramble the rest so length stays similar."""
    if not url or url.startswith("#") or url.startswith("file:"):
        # internal anchors / local — scramble path-like parts only if external-ish
        if url.startswith("#"):
            return "#" + scramble_text(url[1:], rng) if len(url) > 1 else url
    m = re.match(r"^([a-zA-Z][a-zA-Z0-9+.-]*:)(.*)$", url)
    if m:
        return m.group(1) + scramble_text(m.group(2), rng)
    return scramble_text(url, rng)


# ---------------------------------------------------------------------------
# EMF / WMF / SVG placeholders
# ---------------------------------------------------------------------------

EMF_SIGNATURE = 0x464D4520  # " EMF"
EMR_HEADER = 1
EMR_EOF = 14
EMR_SETMAPMODE = 17
EMR_SELECTOBJECT = 37
EMR_RECTANGLE = 43
EMR_CREATEBRUSHINDIRECT = 39
MM_ANISOTROPIC = 8
BS_SOLID = 0
WHITE_BRUSH = 0  # stock object


def parse_emf_size(data: bytes) -> tuple[int, int, tuple[int, int, int, int], tuple[int, int, int, int]] | None:
    """Return (pixel_w, pixel_h, bounds, frame) from EMF header, or None."""
    if len(data) < 88:
        return None
    i_type, n_size = struct.unpack_from("<II", data, 0)
    if i_type != EMR_HEADER:
        return None
    bounds = struct.unpack_from("<iiii", data, 8)  # L T R B
    frame = struct.unpack_from("<iiii", data, 24)
    sig = struct.unpack_from("<I", data, 40)[0]
    if sig != EMF_SIGNATURE:
        return None
    left, top, right, bottom = bounds
    w = max(1, abs(right - left))
    h = max(1, abs(bottom - top))
    # Prefer device bounds; if degenerate, derive from frame (0.01 mm)
    if w <= 1 and h <= 1:
        fl, ft, fr, fb = frame
        # assume ~96 dpi ≈ 26.46 hundredths-mm per pixel… frame is 0.01mm
        # 1 inch = 2540 units of 0.01mm; at 96 dpi → 2540/96 ≈ 26.458 per px
        w = max(1, int(round(abs(fr - fl) / 26.458)))
        h = max(1, int(round(abs(fb - ft) / 26.458)))
    return w, h, bounds, frame


def make_emf_placeholder(
    bounds: tuple[int, int, int, int],
    frame: tuple[int, int, int, int],
    rng: random.Random,
) -> bytes:
    """Minimal valid EMF: gray-ish filled rectangle, same bounds/frame."""
    left, top, right, bottom = bounds
    if right <= left:
        right = left + 100
    if bottom <= top:
        bottom = top + 100
    fl, ft, fr, fb = frame
    if fr <= fl:
        fr = fl + 1000
    if fb <= ft:
        fb = ft + 1000

    # Build records then patch nBytes / nRecords in header
    records: list[bytes] = []

    # Placeholder header (nBytes/nRecords filled later); nSize=88
    header = bytearray(88)
    struct.pack_into("<II", header, 0, EMR_HEADER, 88)
    struct.pack_into("<iiii", header, 8, left, top, right, bottom)
    struct.pack_into("<iiii", header, 24, fl, ft, fr, fb)
    struct.pack_into("<I", header, 40, EMF_SIGNATURE)
    struct.pack_into("<I", header, 44, 0x10000)  # nVersion
    # nBytes @48, nRecords @52 — patched later
    struct.pack_into("<HH", header, 56, 1, 0)  # nHandles=1 (plus stock), reserved
    struct.pack_into("<II", header, 60, 0, 0)  # description
    struct.pack_into("<I", header, 68, 0)  # palette entries
    pw = max(1, abs(right - left))
    ph = max(1, abs(bottom - top))
    struct.pack_into("<ii", header, 72, pw, ph)  # szlDevice
    struct.pack_into("<ii", header, 80, max(1, pw * 25 // 96), max(1, ph * 25 // 96))

    # SETMAPMODE ANISOTROPIC
    records.append(struct.pack("<III", EMR_SETMAPMODE, 12, MM_ANISOTROPIC))

    # CREATEBRUSHINDIRECT — light gray solid brush (handle index 1)
    # EMR_CREATEBRUSHINDIRECT: iType, nSize, ihBrush, logbrush(lbStyle, lbColor, lbHatch)
    gray = 0x00B4B4B4 ^ (rng.randint(0, 7) << 8)  # COLORREF 0x00BBGGRR
    records.append(
        struct.pack(
            "<IIIIII",
            EMR_CREATEBRUSHINDIRECT,
            24,
            1,
            BS_SOLID,
            gray & 0xFFFFFF,
            0,
        )
    )
    # SELECTOBJECT brush
    records.append(struct.pack("<III", EMR_SELECTOBJECT, 12, 1))
    # RECTANGLE
    records.append(
        struct.pack("<II iiii", EMR_RECTANGLE, 24, left, top, right, bottom)
    )
    # EMR_EOF (minimal 20 bytes)
    eof = struct.pack("<IIIII", EMR_EOF, 20, 0, 0, 20)
    records.append(eof)

    body = b"".join(records)
    n_records = 1 + len(records)  # header + others
    n_bytes = 88 + len(body)
    struct.pack_into("<II", header, 48, n_bytes, n_records)
    return bytes(header) + body


def parse_wmf_size(data: bytes) -> tuple[int, int, bytes | None] | None:
    """
    Return (width, height, placeable_header_or_None).
    Width/height in metafile units (for placeable) or defaults.
    """
    placeable = None
    off = 0
    if len(data) >= 22 and struct.unpack_from("<I", data, 0)[0] == 0x9AC6CDD7:
        placeable = data[:22]
        left, top, right, bottom = struct.unpack_from("<hhhh", data, 6)
        inch = struct.unpack_from("<H", data, 14)[0] or 96
        w = max(1, abs(right - left))
        h = max(1, abs(bottom - top))
        # convert to pixels roughly
        px_w = max(1, int(round(w * 96 / inch)))
        px_h = max(1, int(round(h * 96 / inch)))
        return px_w, px_h, placeable
    # non-placeable: try standard header
    if len(data) >= 18:
        # META_HEADER: mtType(2), mtHeaderSize(2), mtVersion(2), mtSize(4), ...
        return 100, 100, None
    return None


def _wmf_checksum(header22: bytes) -> int:
    """XOR checksum for placeable WMF header (words 0..9, checksum word cleared)."""
    words = list(struct.unpack_from("<11H", header22))
    words[10] = 0
    cs = 0
    for w in words:
        cs ^= w
    return cs & 0xFFFF


def make_wmf_placeholder(
    width_px: int,
    height_px: int,
    placeable: bytes | None,
    rng: random.Random,
) -> bytes:
    """Minimal placeable WMF: empty / background box."""
    inch = 96
    if placeable and len(placeable) >= 22:
        left, top, right, bottom = struct.unpack_from("<hhhh", placeable, 6)
        inch = struct.unpack_from("<H", placeable, 14)[0] or 96
    else:
        left = top = 0
        right = max(1, width_px)
        bottom = max(1, height_px)

    # Placeable header
    ph = bytearray(22)
    struct.pack_into("<I", ph, 0, 0x9AC6CDD7)
    struct.pack_into("<h", ph, 4, 0)
    struct.pack_into("<hhhh", ph, 6, left, top, right, bottom)
    struct.pack_into("<H", ph, 14, inch)
    struct.pack_into("<I", ph, 16, 0)
    struct.pack_into("<H", ph, 20, _wmf_checksum(bytes(ph)))

    # Standard metafile header (18 bytes) + records
    # mtType=1 (memory), mtHeaderSize=9 (in words), mtVersion=0x300, mtSize in words
    records: list[bytes] = []

    # META_SETWINDOWEXT (size in words incl. record header)
    # record: rdSize(4) rdFunction(2) ... params
    # Function SETWINDOWEXT = 0x020C, params: height, width (yes, y then x)
    y_ext = max(1, abs(bottom - top))
    x_ext = max(1, abs(right - left))
    records.append(struct.pack("<IHhh", 4, 0x020C, y_ext, x_ext))  # size in WORDS = 4

    # META_SETWINDOWORG 0x020B
    records.append(struct.pack("<IHhh", 4, 0x020B, top, left))

    # META_CREATEBRUSHINDIRECT 0x02FC
    # record size in WORDS includes size+function fields; params: style(2)+color(4)+hatch(2)
    color = 0x00B0B0B0 ^ (rng.randint(0, 5) << 8)
    records.append(
        struct.pack("<IH", 7, 0x02FC) + struct.pack("<HIH", 0, color & 0xFFFFFF, 0)
    )

    # META_SELECTOBJECT 0x012D — object index 0
    records.append(struct.pack("<IHh", 4, 0x012D, 0))

    # META_RECTANGLE 0x041B — bottom, right, top, left (order!)
    records.append(
        struct.pack("<IHhhhh", 7, 0x041B, bottom, right, top, left)
    )

    # META_EOF 0x0000
    records.append(struct.pack("<IH", 3, 0x0000))

    body = b"".join(records)
    # mtSize = header(9 words) + body words
    mt_size = 9 + len(body) // 2
    mf_header = struct.pack(
        "<HHHIHH",
        1,  # mtType disk/memory — 1 = memory metafile
        9,  # mtHeaderSize words
        0x0300,  # mtVersion
        mt_size,
        1,  # mtNoObjects
        max(len(body), 22),  # mtMaxRecord (bytes is wrong but ok-ish; should be words of max record)
    )
    # Fix mtMaxRecord: max record size in words
    max_rec = max((len(r) // 2 for r in records), default=3)
    mf_header = struct.pack("<HHHIHH", 1, 9, 0x0300, mt_size, 1, max_rec)

    return bytes(ph) + mf_header + body


def parse_svg_size(data: bytes) -> tuple[int, int]:
    try:
        text = data.decode("utf-8", errors="replace")
        root = ET.fromstring(text)
    except ET.ParseError:
        return 100, 100
    w = root.get("width") or ""
    h = root.get("height") or ""
    vb = root.get("viewBox") or root.get("viewbox") or ""

    def px(val: str, fallback: int) -> int:
        if not val:
            return fallback
        m = re.match(r"^\s*([0-9.]+)", val)
        return max(1, int(float(m.group(1)))) if m else fallback

    if vb:
        parts = vb.replace(",", " ").split()
        if len(parts) == 4:
            return max(1, int(float(parts[2]))), max(1, int(float(parts[3])))
    return px(w, 100), px(h, 100)


def make_svg_placeholder(width: int, height: int, rng: random.Random) -> bytes:
    g = 180 + rng.randint(-8, 8)
    svg = (
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
        f'viewBox="0 0 {width} {height}">'
        f'<rect width="100%" height="100%" fill="rgb({g},{g},{g})"/>'
        f'<line x1="0" y1="0" x2="{width}" y2="{height}" stroke="#888" stroke-width="2"/>'
        f'<line x1="{width}" y1="0" x2="0" y2="{height}" stroke="#888" stroke-width="2"/>'
        f"</svg>"
    )
    return svg.encode("utf-8")


def decompress_gz_image(data: bytes) -> bytes | None:
    import gzip
    import zlib

    for loader in (
        lambda b: gzip.decompress(b),
        lambda b: zlib.decompress(b),
        lambda b: zlib.decompress(b, -zlib.MAX_WBITS),
    ):
        try:
            return loader(data)
        except Exception:
            continue
    return None


def compress_like_emz(raw: bytes) -> bytes:
    """EMZ/WMZ are gzip-compressed EMF/WMF."""
    import gzip

    buf = io.BytesIO()
    with gzip.GzipFile(fileobj=buf, mode="wb") as gz:
        gz.write(raw)
    return buf.getvalue()


# ---------------------------------------------------------------------------
# Raster placeholders
# ---------------------------------------------------------------------------


def image_size(data: bytes) -> tuple[int, int] | None:
    try:
        with Image.open(io.BytesIO(data)) as im:
            return im.size
    except Exception:
        return None


def make_raster_placeholder(width: int, height: int, fmt: str, rng: random.Random) -> bytes:
    width = max(1, width)
    height = max(1, height)
    base = 180 + rng.randint(-8, 8)
    img = Image.new("RGB", (width, height), (base, base, base))
    draw = ImageDraw.Draw(img)
    margin = max(1, min(width, height) // 20)
    draw.rectangle(
        [margin, margin, width - 1 - margin, height - 1 - margin],
        outline=(120, 120, 120),
        width=max(1, min(width, height) // 80),
    )
    draw.line(
        [(margin, margin), (width - 1 - margin, height - 1 - margin)],
        fill=(140, 140, 140),
        width=max(1, min(width, height) // 100),
    )
    draw.line(
        [(width - 1 - margin, margin), (margin, height - 1 - margin)],
        fill=(140, 140, 140),
        width=max(1, min(width, height) // 100),
    )
    buf = io.BytesIO()
    fmt_u = fmt.upper()
    if fmt_u in ("JPG", "JPEG"):
        img.save(buf, format="JPEG", quality=85)
    elif fmt_u == "GIF":
        img.convert("P", palette=Image.ADAPTIVE).save(buf, format="GIF")
    elif fmt_u in ("TIF", "TIFF"):
        img.save(buf, format="TIFF")
    elif fmt_u == "BMP":
        img.save(buf, format="BMP")
    elif fmt_u == "WEBP":
        img.save(buf, format="WEBP")
    else:
        img.save(buf, format="PNG")
    return buf.getvalue()


def replace_media(name: str, data: bytes, rng: random.Random) -> tuple[bytes, str]:
    """
    Replace media bytes. Returns (new_bytes, status)
    status: replaced | wiped | unchanged
    """
    ext = Path(name).suffix.lower()

    if ext in RASTER_EXTS:
        size = image_size(data)
        if not size:
            return b"\x00" * len(data), "wiped"
        w, h = size
        fmt = "JPEG" if ext in (".jpg", ".jpeg") else ext.lstrip(".").upper()
        if fmt == "JPG":
            fmt = "JPEG"
        return make_raster_placeholder(w, h, fmt, rng), "replaced"

    if ext == ".emf":
        parsed = parse_emf_size(data)
        if parsed:
            _w, _h, bounds, frame = parsed
            return make_emf_placeholder(bounds, frame, rng), "replaced"
        return make_emf_placeholder((0, 0, 100, 100), (0, 0, 2646, 2646), rng), "replaced"

    if ext == ".wmf":
        parsed = parse_wmf_size(data)
        if parsed:
            pw, ph, placeable = parsed
            return make_wmf_placeholder(pw, ph, placeable, rng), "replaced"
        return make_wmf_placeholder(100, 100, None, rng), "replaced"

    if ext in (".emz", ".wmz"):
        raw = decompress_gz_image(data)
        if raw:
            inner_ext = ".emf" if ext == ".emz" else ".wmf"
            new_raw, _ = replace_media(name[: -len(ext)] + inner_ext, raw, rng)
            return compress_like_emz(new_raw), "replaced"
        return b"\x00" * len(data), "wiped"

    if ext == ".svg":
        w, h = parse_svg_size(data)
        return make_svg_placeholder(w, h, rng), "replaced"

    # unknown media — wipe bytes, keep length (layout refs unchanged)
    if data:
        return b"\x00" * len(data), "wiped"
    return data, "unchanged"


# ---------------------------------------------------------------------------
# XML sanitization
# ---------------------------------------------------------------------------


def scramble_sensitive_attrs(elem: ET.Element, rng: random.Random) -> None:
    for attr, val in list(elem.attrib.items()):
        local = _local(attr)
        if local in ("descr", "title", "alt", "tooltip"):
            if val:
                elem.set(attr, scramble_text(val, rng))
        elif local == "name" and val and any(c.isalpha() for c in val) and len(val) > 2:
            # keep generic names like "Picture 1" structure
            elem.set(attr, scramble_text(val, rng))


def scramble_xml_tree(root: ET.Element, rng: random.Random, meta_mode: bool = False) -> None:
    for elem in root.iter():
        local = _local(elem.tag)
        scramble_sensitive_attrs(elem, rng)

        if elem.text and elem.text.strip():
            if meta_mode or local in TEXT_LOCAL_NAMES or local in META_TEXT_LOCAL:
                elem.text = scramble_text(elem.text, rng)

        if elem.tail and elem.tail.strip() and (
            meta_mode or local in TEXT_LOCAL_NAMES or local in META_TEXT_LOCAL
        ):
            elem.tail = scramble_text(elem.tail, rng)


def scramble_rels(data: bytes, rng: random.Random) -> bytes:
    try:
        root = ET.fromstring(data)
    except ET.ParseError:
        return data
    for elem in root.iter():
        if _local(elem.tag) == "Relationship":
            target = elem.get("Target")
            mode = elem.get("TargetMode", "")
            if target and (mode == "External" or re.match(r"^[a-zA-Z][a-zA-Z0-9+.-]*:", target)):
                elem.set("Target", scramble_url(target, rng))
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


def scramble_xml_bytes(data: bytes, rng: random.Random, meta_mode: bool = False) -> bytes:
    try:
        root = ET.fromstring(data)
    except ET.ParseError:
        # not xml / corrupt — wipe if looks like plaintext
        return data
    scramble_xml_tree(root, rng, meta_mode=meta_mode)
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


def classify_part(name: str) -> str:
    n = name.replace("\\", "/").lstrip("/").lower()
    if n.endswith(".rels"):
        return "rels"
    if n.startswith("docprops/"):
        if n.endswith((".jpeg", ".jpg", ".png", ".emf", ".wmf")):
            return "media"
        return "meta"
    if n.startswith("word/media/") or n.startswith("word/embeddings/"):
        return "media"
    if n.startswith("customxml/"):
        return "xml_text"
    if n.endswith(".xml") and (
        n.startswith("word/")
        or n == "[content_types].xml"
        or n.startswith("customxml/")
    ):
        if n == "[content_types].xml":
            return "passthrough"
        return "xml_text"
    if n.startswith("word/") and "/media/" not in n and n.endswith(
        (".bin", ".xlsx", ".docx", ".doc", ".ole")
    ):
        return "wipe"
    return "passthrough"


# ---------------------------------------------------------------------------
# Package
# ---------------------------------------------------------------------------


def sanitize_docx(src: Path, dst: Path, seed: int | None = None) -> dict:
    rng = random.Random(seed)
    stats = {
        "xml_parts": 0,
        "images_replaced": 0,
        "images_wiped": 0,
        "rels": 0,
        "meta": 0,
        "embeddings_wiped": 0,
        "passthrough": 0,
    }

    with zipfile.ZipFile(src, "r") as zin, zipfile.ZipFile(
        dst, "w", compression=zipfile.ZIP_DEFLATED
    ) as zout:
        for info in zin.infolist():
            name = info.filename
            data = zin.read(name)
            kind = classify_part(name)
            out = data

            if kind == "xml_text":
                out = scramble_xml_bytes(data, rng, meta_mode=False)
                stats["xml_parts"] += 1
            elif kind == "meta":
                out = scramble_xml_bytes(data, rng, meta_mode=True)
                # also scramble bare text in property values under custom props
                stats["meta"] += 1
            elif kind == "rels":
                out = scramble_rels(data, rng)
                stats["rels"] += 1
            elif kind == "media":
                lower = name.replace("\\", "/").lower()
                if "/embeddings/" in lower:
                    out = b"\x00" * len(data) if data else data
                    stats["embeddings_wiped"] += 1
                else:
                    out, status = replace_media(name, data, rng)
                    if status == "replaced":
                        stats["images_replaced"] += 1
                    elif status == "wiped":
                        stats["images_wiped"] += 1
                    else:
                        stats["passthrough"] += 1
            elif kind == "wipe":
                out = b"\x00" * len(data) if data else data
                stats["embeddings_wiped"] += 1
            else:
                stats["passthrough"] += 1

            new_info = zipfile.ZipInfo(filename=name, date_time=info.date_time)
            new_info.compress_type = zipfile.ZIP_DEFLATED
            new_info.external_attr = info.external_attr
            zout.writestr(new_info, out)

    return stats


def default_output(src: Path) -> Path:
    return src.with_name(src.stem + ".sanitized" + src.suffix)


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(
        description="Fully sanitize a DOCX: scramble text, replace all images "
        "(including EMF/WMF/SVG), wipe embeddings, clear metadata — template kept."
    )
    p.add_argument("input", type=Path, help="Source .docx")
    p.add_argument("-o", "--output", type=Path, default=None, help="Output path")
    p.add_argument("--seed", type=int, default=None, help="RNG seed")
    args = p.parse_args(argv)

    src: Path = args.input
    if not src.is_file():
        print(f"File not found: {src}", file=sys.stderr)
        return 1
    if src.suffix.lower() not in {".docx", ".docm"}:
        print("Expected a .docx file", file=sys.stderr)
        return 1

    dst = args.output or default_output(src)
    if dst.resolve() == src.resolve():
        print("Output must differ from input", file=sys.stderr)
        return 1

    try:
        stats = sanitize_docx(src, dst, seed=args.seed)
    except zipfile.BadZipFile:
        print("Not a valid DOCX/ZIP archive", file=sys.stderr)
        return 1

    print(f"Wrote: {dst}")
    print(
        f"XML parts: {stats['xml_parts']}, "
        f"images replaced: {stats['images_replaced']}, "
        f"images wiped: {stats['images_wiped']}, "
        f"embeddings wiped: {stats['embeddings_wiped']}, "
        f"rels: {stats['rels']}, meta: {stats['meta']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
