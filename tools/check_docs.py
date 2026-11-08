#!/usr/bin/env python3
"""Check local links and anchors in a built Zensical site. No network access."""

import argparse
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit


class Page(HTMLParser):
    def __init__(self, source):
        super().__init__()
        self.links = []
        self.ids = set()
        self.feed(source)

    def handle_starttag(self, tag, attributes):
        attrs = dict(attributes)
        if "id" in attrs:
            self.ids.add(attrs["id"])
        for key in ("href", "src", "poster"):
            if key in attrs:
                self.links.append(attrs[key])


def check(root):
    root = root.resolve()
    pages = {p: Page(p.read_text(encoding="utf-8")) for p in root.rglob("*.html")}
    if not pages:
        raise ValueError(f"No HTML pages found under {root}")
    errors = []
    count = 0
    for path, page in pages.items():
        for link in page.links:
            url = urlsplit(link)
            if url.scheme or url.netloc or url.path.startswith("/"):
                continue
            target = (path.parent / unquote(url.path)).resolve() if url.path else path
            if target.is_dir():
                target /= "index.html"
            count += 1
            if not target.is_relative_to(root) or not target.is_file():
                errors.append(f"{path.relative_to(root)}: missing {link}")
            elif url.fragment and target in pages and unquote(url.fragment) not in pages[target].ids:
                errors.append(f"{path.relative_to(root)}: missing anchor {link}")
    if errors:
        raise ValueError("\n".join(errors))
    print(f"Checked {len(pages)} HTML pages and {count} local links/anchors")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("site", type=Path)
    check(parser.parse_args().site)
