# Mobile input fix round 1

The goal is real Horizon browser gameplay with reliable touch and keyboard input,
including multiplayer through private Tailscale. This round addresses both P2
findings from the Codex mobile review. It starts at clean pushed head
`2623ca8d875d7a5b81d0c17b61bae04699d3cd01` on `codex/horizon-web`.

| Finding | Baseline | Treatment | State |
| --- | --- | --- | --- |
| Separate Jump taps become flight during a render stall | Normal control stays grounded; stalled taps enable flight and subsequent ascent reaches the observer | Separate stalled taps stay grounded; holding Jump does not produce lasting flight | Implemented and parent browser-verified; fresh review pending |
| Deferred reset erases fresh input after focus returns | Fresh touch and W each move zero blocks; releasing and repressing touch moves 3.2341 blocks | Fresh touch moves 3.4100 blocks; fresh W moves 3.4101 blocks, with observer updates | Implemented and parent browser-verified; fresh review pending |

## Correction

`PlayerInput` retains a timestamp for each aggregate Jump transition. Keyboard
and touch owners feed the same transition queue. Presses separated by at least
350 ms clear the native flight gesture history before the existing motion kernel
consumes the new press. Short taps remain queued, and valid fast double taps
still use native flight behavior. No gameplay engine or native physics changed.

`BrowserInput` owns shared keyboard sets alongside pointer state. Lifecycle
handlers clear these sources immediately. No later draw clears input again.
New post-focus input therefore survives until the simulation consumes it.
Model the Domain changed jump history from booleans to timed transitions with
explicit owners. Fix Root Causes removed the delayed reset rather than filtering
fresh input in the draw loop.

## Reproduction and artifact checks

The original review is preserved in `mobile-adversarial-round1.md`. The parent
cloned both exact repro scripts into a fresh temporary directory and changed
only their test URL to the disposable local bridge. Commands ran from the source
worktree.

```sh
node /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-mobile-fix-round1.u6l6e7lc/before/focused-v3.cjs
node /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-mobile-fix-round1.u6l6e7lc/before/trusted-focus-v2.cjs
node /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-mobile-fix-round1.u6l6e7lc/after/focused-v3.cjs
node /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-mobile-fix-round1.u6l6e7lc/after/trusted-focus-v2.cjs
```

`mobile-input-repros.json` preserves input timestamps, before/after client state,
trusted focus events, and server-delivered observer positions. Explicit parent
assertions checked both baseline failures and both corrected outcomes. These
focused treatment runs used the first correction build. A later Clippy-only
conditional flattening preceded the final build and full regression driver.

The final expanded `tools/web_touch_check.cjs` passed. Its stalled taps were
722.2 ms apart and flight stayed false. It then accepted fresh touch and W after
two real Chrome tab switches with trusted blur/focus events. Valid double-tap
flight, flight rise/descent, block placement/removal, cleanup, and four viewport
sizes still passed. The parent inspected `placed.png` and saw the gold block and
touch controls in the actual world. Captures remain private in `.local/`.

The test server was created with `tools/run_web_test_server.py --resources
.local/web-owned/26.3 --port 25587`. The bridge bound `127.0.0.1:18787`, forwarded
to `127.0.0.1:25587`, and allowed origin `http://127.0.0.1:8766` for `webproof`.
The server tool created fresh game, launcher, data, and configuration directories.
The four protected live services and Tailscale configuration were unchanged.

```sh
export PATH=/Users/test/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH
export RUSTC=/Users/test/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc
cargo fmt -p horizon-web
cargo test -p horizon-web --lib
cargo clippy -p horizon-web --target wasm32-unknown-unknown -- -D warnings -A clippy::manual_range_contains
python3 tools/build_web.py
node --check tools/web_touch_check.cjs
python3 tools/check_public_tree.py
git diff --check
export HORIZON_PLAYWRIGHT=/Users/test/.npm/_npx/705bc6b22212b352/node_modules/playwright
export HORIZON_WEB_URL='http://127.0.0.1:8766/?world=webproof&bridge=ws://127.0.0.1:18787'
HORIZON_WEB_PROOF=.local/web-mobile-fix-round1 node tools/web_touch_check.cjs
HORIZON_WEB_NAME_PREFIX=FixMob1 HORIZON_WEB_PROOF=.local/web-mobile-fix-desktop node tools/web_play_check.cjs
HORIZON_WEB_PROOF=.local/web-mobile-fix-tick node tools/web_tick_check.cjs
```

All six input unit tests pass. Scoped Clippy passes with the unchanged aquifer
`manual_range_contains` baseline exception. The first Clippy run identified a
new collapsible conditional in cancellation cleanup. It was flattened before
the passing run. No warning suppression was added.

The desktop two-client driver passed movement, placement, reconnect, and removal.
The tick driver passed normal and delayed-render windows. These are functional
cadence checks, not performance measurements. Machine-readable receipts are
`mobile-input-fix-touch.json`, `mobile-input-fix-desktop.json`, and
`mobile-input-fix-tick.json`.

`mobile-input-fix-served.json` confirms that HTML, JavaScript, and wasm served
through private Tailscale match local bytes. The final wasm SHA-256 is
`aaab226ef53e6e9abf33898e3223ab822fe9dbf42211f1aba914ce2ea63e14d0`.
The final full gameplay drivers used the disposable local world. Served-byte
identity is not an additional Tailscale gameplay run.

## Remaining scope

The first fix round still needs a fresh runtime review. Cross-family review is
blocked by unavailable Claude providers. Actual Safari, phone, and Duo testing
remains blocked by the occupied device session. Offline browser worlds, full
native parity, public hosting, and execution inside X remain unfinished. No PR,
merge, public deployment, device action, or X post occurred.
