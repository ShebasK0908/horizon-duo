# Parent browser verification

The goal is actual Horizon browser gameplay and multiplayer through the private
Tailscale route. The tested source is `1fc53d53515e41a1d9d90cbbe1dff7ed85d3490b`.
The branch was clean and synchronized with `fork/codex/horizon-web` before these checks.

## Touch artifact

The parent reran `tools/web_touch_check.cjs` through private Tailscale with a fresh
profile. The driver exited successfully. It verified movement and release,
simultaneous movement and look, cancellation, lost capture, keyboard and touch
ownership, multiple pointer owners, retained quick jump, creative flight,
authoritative gold placement and removal, and four viewport sizes. Blur and
visibility checks dispatch DOM events. They do not prove real phone backgrounding.

The parent inspected the fresh 390x844 capture. It shows the rendered Horizon
world, crosshair, material selector, and edge controls inside the viewport.
The capture stays private under `.local/web-touch-parent-verify/`.

```sh
HORIZON_PLAYWRIGHT=/Users/test/.npm/_npx/705bc6b22212b352/node_modules/playwright \
HORIZON_WEB_BUNDLE=.local/web-owned/play-resources.zip \
HORIZON_WEB_URL='https://tests-mac-studio.tail935cc1.ts.net:8491/?world=webproof&bridge=wss%3A%2F%2Ftests-mac-studio.tail935cc1.ts.net%3A8492' \
HORIZON_WEB_PROOF=.local/web-touch-parent-verify node tools/web_touch_check.cjs
```

`mobile-parent-touch-results.json` preserves the result.
`mobile-parent-served-artifacts.json` records byte equality between the served
HTML, JavaScript, and WebAssembly and the local artifacts. The wasm SHA-256 is
`2afe5825e9baca8c3ba29b746c77582fd553423e8547859f1c9c2a5773955d3c`.

## Source review

The parent inspected pointer ownership, retained input transitions, keyboard
merging, reset boundaries, CSS placement, and both startup callers. The Codex
comment review found zero newly introduced comments, docstrings, or suppressions
in the nine scoped implementation, configuration, and harness files. No comment
deletion, restoration, workaround fix, or constraint encoding was needed.

The adversarial runtime review remains pending at this checkpoint. Cross-family
review remains blocked by the unavailable Claude providers. Actual Safari and
Duo evidence remains blocked by the occupied device session. Native suites were
not rerun because this checkpoint changes no shared engine source. Offline
browser worlds, full native parity, and execution inside X remain unfinished.

## Cross-origin iframe artifact

`tools/web_embed_check.cjs` runs a real local HTTP parent on a temporary loopback
port. It embeds Horizon from the existing HTTP origin in a 720x720 iframe.
The sandbox grants scripts, same-origin behavior, pointer lock, and forms.
The parent and game origins differ. No browser permission checks are disabled.

The isolated run passed. Keyboard input moved the player. The native server
acknowledged gold state 3830 at `[0,-60,3]`. The parent inspected `frame.png`
after the mesh queue drained and saw the gold block in the framed world.
`x-iframe-browser.json` preserves the state and browser logs. The owned-texture
capture stays private under `.local/web-embed-isolated-verify/`.

```sh
python3 tools/run_web_test_server.py --resources .local/web-owned/26.3 --port 25587
target/debug/horizon-web-bridge --bind 127.0.0.1:18787 --upstream 127.0.0.1:25587 --world webproof --origin http://127.0.0.1:8766
HORIZON_PLAYWRIGHT=/Users/test/.npm/_npx/705bc6b22212b352/node_modules/playwright \
HORIZON_WEB_URL='http://127.0.0.1:8766/?world=webproof&bridge=ws://127.0.0.1:18787' \
HORIZON_WEB_PROOF=.local/web-embed-isolated-verify node tools/web_embed_check.cjs
```

The first two commands run in separate terminals. The server tool creates fresh
game, launcher, data, and configuration directories. The owned test server
exited 0 after `stop`; the owned bridge was terminated. Existing services on
25575, 8766, 8767, and 18767 were left running.

Early probes used a Playwright-intercepted parent. Chrome rejected its loopback
iframe with `ERR_BLOCKED_BY_LOCAL_NETWORK_ACCESS_CHECKS`. A real HTTP parent
resolved that setup failure. The reusable check then hit
`multiplayer.disconnect.server_full` on the shared four-player world during
concurrent verification. The final command used the disposable server and bridge
above. Those failed runs do not count as passes. Their logs remain in `/tmp` and
`.local/web-embed-parent-verify/`.

This is Chrome framing evidence, not X execution. No public endpoint, Player
Card acceptance, X sandbox configuration, X post, or phone browser was verified.
The X source post returned HTTP 403. Rosebud's first-party engineering report
describes Player Card iframes but cannot prove Horizon support.

The new harness passed `node --check`, `git diff --check`, and
`python3 tools/check_public_tree.py`. It changes no game or native engine code.
