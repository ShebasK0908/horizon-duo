#!/usr/bin/env python3
"""Package player-owned prepared resources for browser rendering validation."""
import argparse
import json
from pathlib import Path
import re
import zipfile

ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("resources", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    version = (ROOT / "crates/horizon-core/src/version.rs").read_text()
    expected = int(re.search(r"DATA_VERSION: i32 = (\d+)", version)[1])
    manifest = json.loads((args.resources / "horizon-resources.json").read_text())
    if manifest.get("data_version") != expected or manifest.get("schema") != 7:
        raise SystemExit("Prepare resources for Horizon's pinned version before importing them.")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    top = {"facts.json", "horizon-resources.json", "block-tags.json", "model-states.json"}
    with zipfile.ZipFile(args.output, "w", zipfile.ZIP_DEFLATED, compresslevel=1) as archive:
        for file in sorted(args.resources.rglob("*")):
            relative = file.relative_to(args.resources)
            render = len(relative.parts) > 2 and relative.parts[0] == "assets" and relative.parts[2] in {"models", "textures", "blockstates"}
            if file.is_file() and (relative.as_posix() in top or render):
                archive.write(file, relative.as_posix())
    print(args.output)

if __name__ == "__main__":
    main()
