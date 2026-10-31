# Private Player Card preparation

The goal is a shareable Horizon browser world with multiplayer and a possible
X integration. This checkpoint prepares a concrete static card candidate. It
does not establish execution inside X. Source started at clean pushed head
`728012fa214cf2cd7a27b1a07f96af380d82d75d`.

## Shape and choice

The candidate has two explicit absolute HTTPS URLs, one for the player and one
for the preview, plus a title. `tools/web_card.py` renders a static HTML page.
Its Player Card metadata points directly to the existing game. The visible play
link opens that same URL. No additional iframe or request-time metadata server
was added. A request-time server was rejected because this candidate needs one
explicit world configuration and the existing static host can serve it.
Laziness Protocol kept this as a small generator, rather than a new web service.

`web/player-card.svg` is original geometric artwork, rendered once by Chrome
into `web/player-card.png` at 1200x630. The parent inspected the image. Neither
file contains imported game resources, game screenshots, or proprietary logos.
The SVG source and exact regeneration command are in `docs/web-x-card.md`.

The host-specific output `web/share.html` is ignored. It is currently served
privately at `https://tests-mac-studio.tail935cc1.ts.net:8491/share.html`.
Generating the card did not change the game, wasm, bridges, server, or Tailscale
configuration. The public-host candidate must be regenerated with public URLs.

```sh
python3 tools/web_card.py \
  --player-url 'https://tests-mac-studio.tail935cc1.ts.net:8491/?world=webproof&bridge=wss%3A%2F%2Ftests-mac-studio.tail935cc1.ts.net%3A8492' \
  --image-url 'https://tests-mac-studio.tail935cc1.ts.net:8491/player-card.png'
python3 -m py_compile tools/web_card.py
python3 tools/check_public_tree.py
git diff --check
```

## Checks

The HTTPS boundary rejects HTTP, JavaScript URLs, credentials including empty
userinfo, invalid ports, and whitespace. An ad hoc import of `web_card.py`
asserted those failures and accepted the intended HTTPS game URL. It also checked
HTML escaping for a title containing `<script>` and a player URL containing `&`.
These checks concern the generated document, not live URL reachability.

The parent fetched `/share.html` through private Tailscale with `Twitterbot/1.0`
and an ordinary user agent. Both returned HTTP 200 and bytes identical to the
local generated document. Python's HTML parser decoded the Player Card and Open
Graph tags, exact world URL, width 720, height 720, preview URL, and play link.
The PNG returned `image/png`, dimensions 1200x630, and local-identical bytes.
`x-card-http-results.json` preserves the values.

A fresh Chrome page at 390x844 loaded the candidate and decoded its preview.
The parent inspected the generated preview image. The browser followed
**Play Horizon** and waited for the actual game's visible resource-import input.
The final game URL matched the configured world and gateway, with no page errors.
`x-card-browser.json` preserves that result. The page capture is private under
`.local/x-card/page.png`. No resource import or new gameplay assertion was needed
for this metadata-only change.

The module docstring supplies the CLI help description. No new suppressions or
application workaround comments were added. The generator was implemented
in-thread because its static output and CLI boundary are small. The separate
mobile runtime reviewer remains scoped to the earlier input correction.

## Limits

A Twitterbot user-agent string on a request from a tailnet machine is not an X
crawler request. The candidate's URLs remain private. No X post, public host,
actual X sandbox, card acceptance, or in-X gameplay was exercised. The first-party
Rosebud report supports an experimental Player Card candidate, not a current X
acceptance contract. The legacy official Player Card URL still redirects to
X's general developer overview.

The public deployment steps are recorded in `docs/web-x-card.md`. The private
resource ZIP stays outside the browser's public document root. Cross-family
review remains blocked by unavailable Claude providers. Actual Safari/Duo proof,
offline browser gameplay, and native parity remain unfinished.
