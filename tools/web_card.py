#!/usr/bin/env python3
"""Generate a static Horizon Player Card candidate for an HTTPS game URL."""
import argparse
from html import escape
from pathlib import Path
from urllib.parse import urlsplit


def https_url(value: str) -> str:
    try:
        url = urlsplit(value)
        valid = (
            url.scheme == "https"
            and url.hostname
            and url.username is None
            and url.password is None
            and not any(c.isspace() for c in value)
        )
        url.port
    except ValueError:
        valid = False
    if not valid:
        raise argparse.ArgumentTypeError("Use an absolute HTTPS URL without credentials or whitespace")
    return value


def page(player_url: str, image_url: str, title: str) -> str:
    description = "A shared Horizon world. Import your prepared resources to play in a browser with WebGPU."
    tags = {
        "twitter:card": "player",
        "twitter:title": title,
        "twitter:description": description,
        "twitter:player": player_url,
        "twitter:player:width": "720",
        "twitter:player:height": "720",
        "twitter:image": image_url,
        "twitter:image:alt": "Horizon. A shared world in your browser.",
    }
    metadata = "\n".join(f'<meta name="{name}" content="{escape(value, quote=True)}">' for name, value in tags.items())
    social = "\n".join(f'<meta property="og:{name}" content="{escape(value, quote=True)}">' for name, value in {
        "type": "website", "title": title, "description": description, "image": image_url,
    }.items())
    return f'''<!doctype html>
<html lang="en">
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{escape(title)}</title>
{metadata}
{social}
<style>
body{{margin:0;background:#101d28;color:#f2f7fa;font:18px system-ui}}
main{{max-width:720px;margin:0 auto;padding:24px}}img{{display:block;width:100%;border-radius:16px}}
h1{{font-size:32px;line-height:1.1}}p{{line-height:1.5;color:#c6d7e3}}
a{{display:inline-block;background:#b4ead5;color:#101d28;padding:14px 24px;border-radius:12px;font-weight:700;text-decoration:none}}
</style>
<main>
<img src="{escape(image_url, quote=True)}" alt="Horizon. A shared world in your browser.">
<h1>{escape(title)}</h1>
<p>{escape(description)}</p>
<a href="{escape(player_url, quote=True)}">Play Horizon</a>
</main>
</html>
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--player-url", type=https_url, required=True)
    parser.add_argument("--image-url", type=https_url, required=True)
    parser.add_argument("--title", default="Horizon multiplayer")
    parser.add_argument("--output", type=Path, default=Path("web/share.html"))
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(page(args.player_url, args.image_url, args.title), encoding="utf-8")
    print(f"Wrote {args.output}. X acceptance and public access remain unverified.")


if __name__ == "__main__":
    main()
