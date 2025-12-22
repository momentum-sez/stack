#!/usr/bin/env python3
"""Render Akoma Ntoso documents to HTML and (optionally) PDF.

HTML rendering uses XSLT (tools/akoma/xslt/akn2html.xsl).

PDF rendering strategy:
- Prefer WeasyPrint if installed (HTML -> PDF).
- Fallback: ReportLab text-only PDF (structure simplified).
"""

from __future__ import annotations
import argparse
import pathlib
from lxml import etree

def render_html(xml_path: pathlib.Path, xslt_path: pathlib.Path, out_html: pathlib.Path) -> None:
    dom = etree.parse(str(xml_path))
    xslt = etree.XSLT(etree.parse(str(xslt_path)))
    result = xslt(dom)
    out_html.write_bytes(etree.tostring(result, pretty_print=True, encoding="utf-8"))

def render_pdf_from_html(html_path: pathlib.Path, out_pdf: pathlib.Path) -> bool:
    try:
        from weasyprint import HTML  # type: ignore
        HTML(filename=str(html_path)).write_pdf(str(out_pdf))
        return True
    except Exception:
        return False

def render_pdf_text(xml_path: pathlib.Path, out_pdf: pathlib.Path) -> None:
    from reportlab.lib.pagesizes import LETTER
    from reportlab.pdfgen import canvas

    c = canvas.Canvas(str(out_pdf), pagesize=LETTER)
    width, height = LETTER
    y = height - 72
    c.setFont("Helvetica", 11)

    # naive extraction of all text nodes
    dom = etree.parse(str(xml_path))
    text = " ".join([t.strip() for t in dom.xpath("//text()") if str(t).strip()])
    # wrap lines
    max_chars = 90
    lines = [text[i:i+max_chars] for i in range(0, len(text), max_chars)]
    for line in lines:
        c.drawString(72, y, line)
        y -= 14
        if y < 72:
            c.showPage()
            c.setFont("Helvetica", 11)
            y = height - 72
    c.save()

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("xml", help="path to Akoma XML")
    ap.add_argument("--out-dir", default="dist/render", help="output directory")
    ap.add_argument("--pdf", action="store_true", help="also produce PDF")
    args = ap.parse_args()

    xml_path = pathlib.Path(args.xml)
    out_dir = pathlib.Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    xslt_path = pathlib.Path(__file__).resolve().parent / "xslt" / "akn2html.xsl"
    html_path = out_dir / (xml_path.stem + ".html")
    render_html(xml_path, xslt_path, html_path)
    print("Wrote", html_path)

    if args.pdf:
        pdf_path = out_dir / (xml_path.stem + ".pdf")
        if not render_pdf_from_html(html_path, pdf_path):
            render_pdf_text(xml_path, pdf_path)
        print("Wrote", pdf_path)

    return 0

if __name__ == "__main__":
    raise SystemExit(main())
