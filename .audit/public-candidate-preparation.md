# Separate single-origin private candidate

This prepares a concrete Horizon game package and separate demo world for later
public validation. It remains private. Public hosting and X posting have not
occurred. The original private world on 8491/8492 and every unrelated Tailscale
route remain unchanged.

The installed Tailscale configuration already uses all three Funnel-capable
ports, 443, 8443 and 10000. Port 8443 serves an unrelated private service on 3212.
Those routes were preserved. A new private Serve origin on 8493 combines the
static package and a world-specific WebSocket path. Native world data is fresh
and separate from the original private world.

| Surface | Owned configuration |
| --- | --- |
| Private game | `https://tests-mac-studio.tail935cc1.ts.net:8493/?world=xproof` |
| Private share candidate | `https://tests-mac-studio.tail935cc1.ts.net:8493/share.html` |
| HTTP | PID 88416, loopback 38877, `.local/web-public-candidate/` |
| WebSocket bridge | PID 88402, loopback 38876, `/world/xproof`, exact private origin 8493 |
| Separate native server | PID 88382, loopback 38875 |
| World data | `.local/web-test-server-coy6pae_/` |

The live managed command sessions are native server 50032, bridge 65940, and HTTP
57386. They remain running for private review. Stop only these owned processes
when retiring the candidate. Remove only the owned Serve port with
`tailscale serve --https=8493 off`; never reset the global configuration.
The original protected PIDs 55502, 88366, 88504 and 9771 remain unchanged.

## Package and setup

The package copies `web/index.html`, `web/pkg/`, and the original preview PNG.
The generator writes its own `share.html` into this separate directory with
player URL `https://tests-mac-studio.tail935cc1.ts.net:8493/?world=xproof`.
No explicit bridge parameter is needed. The existing browser bootstrap derives
the WebSocket origin from the page and sets `/world/xproof`.
The package contains only seven browser artifact/preview files. It contains no
resource ZIP, world data, server configuration, credentials or owned captures.
`public-candidate-package.json` records names, sizes and hashes.

Commands ran from the source worktree, each long-running server in its own
managed session:

```sh
python3 tools/run_web_test_server.py --resources .local/web-owned/26.3 --port 38875
target/debug/horizon-web-bridge --bind 127.0.0.1:38876 --upstream 127.0.0.1:38875 --world xproof --origin https://tests-mac-studio.tail935cc1.ts.net:8493
python3 -m http.server 38877 --bind 127.0.0.1 --directory .local/web-public-candidate
tailscale serve --bg --https=8493 http://127.0.0.1:38877
tailscale serve --bg --https=8493 --set-path=/world/ http://127.0.0.1:38876/world/
```

The test-server tool supplied isolated game, launcher, XDG data/config homes,
no self-update and no audio. To resume this same disposable world after stopping
it, pass `--resume .local/web-test-server-coy6pae_` and the same resources/port.
Before configuring 8493, the parent captured Serve configuration and confirmed
that port was unused. After configuration and gameplay, comparisons excluding
only the owned 8493 entries showed every existing TCP/Web/AllowFunnel entry
unchanged. AllowFunnel for 8493 is false. No public ngrok endpoint was started.

## Verification

Private HTTP returned 200 for share HTML, JavaScript, wasm and preview. A request
for `/resources.zip` returned 404. The wasm matches source artifact SHA-256
`2ab67e89cc87b1eef188fe1e9d57a5b311fb40f721366c3e6c9d92a7de1fb938`.
TLS WebSocket handshakes returned 101 for the correct world/origin, 403 for a
wrong origin, and 403 for a wrong world. The mounted proxy preserves the full
`/world/xproof` path. `public-candidate-http.json` and
`public-candidate-websocket.json` retain those results.

```sh
HORIZON_PLAYWRIGHT=/Users/test/.npm/_npx/705bc6b22212b352/node_modules/playwright \
HORIZON_WEB_URL='https://tests-mac-studio.tail935cc1.ts.net:8493/?world=xproof' \
HORIZON_WEB_NAME_PREFIX=XStage HORIZON_WEB_PROOF=.local/web-public-candidate-proof \
node tools/web_play_check.cjs
```

The two-client Chrome check passed movement convergence, native-server gold
placement seen by both clients, reconnect retaining that edit, and removal
seen by both clients. No page or GPU errors were recorded. The parent inspected
the fresh placed-block capture. Owned-texture captures stay private under
`.local/web-public-candidate-proof/`. `public-candidate-browser.json` records
the actual assertions. A TLS request using a different HTTP Host header also
returned share HTML, which supports a future external reverse proxy. It is not
a test of any public provider.

## Public transition remains gated

A public endpoint could proxy this isolated private origin without reassigning
other services. The installed ngrok CLI supports an HTTPS upstream and
`--upstream-tls-verify`; its existing configuration validates. No endpoint was
started or provider account/limits tested. One possible command, only after
public-deployment authorization, is:

```sh
ngrok http https://tests-mac-studio.tail935cc1.ts.net:8493 --upstream-tls-verify --inspect=false
```

After a real public origin is allocated, add that exact origin to only the owned
bridge while preserving its private origin. Regenerate only the staged share
page with the public player and preview URLs. The game derives its WSS URL from
that public origin. Repeat external HTTP, origin rejection, multiplayer and
iframe checks before a separately authorized X-post check. Public provider
behavior, account limits, X crawler access, card acceptance and in-X execution
remain unverified. The private resource ZIP stays private.

The pending fresh mobile review covers the latest input correction. Cross-family
review remains blocked by unavailable Claude providers. Actual Safari/Duo and
full native parity remain unfinished. Offline implementation is unstarted and
the scope question remains open; no deferral is assumed.
