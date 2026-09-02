#!/usr/bin/env python3
"""
check_links.py

Checks that markdown cross-doc links point to files that exist.
Links starting with http://, https://, or # are skipped.

Exits 0 if all links are valid, non-zero otherwise.
"""
import os
import re
import sys
from pathlib import Path

def main():
    broken = []
    root = Path(__file__).parent.parent.parent
    # Scan all .md files
    for md_file in sorted(root.rglob("*.md")):
        # Skip .archive directories and other synthetic content
        if ".archive" in str(md_file) or ".git" in str(md_file):
            continue

        rel_path = md_file.relative_to(root)
        doc_dir = md_file.parent

        in_code_block = False
        with open(md_file, encoding="utf-8", errors="replace") as f:
            for i, line in enumerate(f, 1):
                # Track fenced code blocks
                if re.match(r"```", line):
                    in_code_block = not in_code_block
                    continue
                if in_code_block:
                    continue
                # Match markdown links [text](url)
                for m in re.finditer(r'\[([^\]]+)\]\(([^)]+)\)', line):
                    target = m.group(2).strip()
                    if not target:
                        continue
                    # Skip http/https/anchors/mailto
                    if target.startswith(("http://", "https://", "#", "mailto:")):
                        continue

                    # Resolve relative to the document's directory
                    target_path = (doc_dir / target).resolve()
                    # Also try resolving relative to root
                    root_target = (root / target).resolve()

                    if not (target_path.exists() or root_target.exists()):
                        broken.append(
                            f"  {rel_path}:{i}: broken link [{m.group(1)}]({target})"
                        )

    if broken:
        print("Broken cross-doc links found:")
        for b in broken:
            print(b)
        return 1

    print("All cross-doc links valid.")
    return 0

if __name__ == "__main__":
    sys.exit(main())
