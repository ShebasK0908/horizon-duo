# Mobile input and browser lifecycle verification

The goal is real Horizon gameplay from the private Tailscale URL, preserving the
native authoritative world and multiplayer. This phase implements browser touch
input and viewport adaptation. It does not establish phone or Safari execution.

The implementation started at clean `aba56c78dd2c2974c64e47c2b448ee8925b1dab3`
on `codex/horizon-web`, synchronized 0/0 with `fork/codex/horizon-web`. The exclusive
worktree is `/Users/test/.t3/scratch/2026-10-25-compile-this-for-the-iphone-cf98d4f8/src/horizon-web`.
The original native iPhone checkout and old touch worktrees were untouched.

The tested source checkpoint is `6627652e0fe6e92de1d3139cbf5edb33b716386d`,
pushed normally to the fork branch. Rebuilding that committed source produced
byte-identical HTML, JavaScript, and wasm to the tested artifact.

## Design and scope

The source-grounded comparison was sequential, as requested. Canvas touch regions
would avoid DOM listeners but require custom accessible targets and safe-area
layout. DOM edge controls let CSS handle presentation and viewport adaptation.
Rust owns the pointer-role map, retained jump transitions, action queue, and source
merging. These feed existing `Keys`, `MotionWorld`, ray picking, and `interact`.
Double-tap jump uses the native flight toggle. No gameplay engine was added.

The Model the Domain principle changed input ownership to typed pointer roles and
queues. Prove It Works changed verification to actual CDP touch and server block
updates. Laziness Protocol kept the lifecycle change to one explicit bootstrap
for the two existing callers, rather than a generic application framework.
The iPhone Duo games skill informed fixed CSS target sizes, outer-edge anchoring,
safe-area insets, and variable aspect ratio. No native hinge API was used.

| Scope item | State | Evidence |
| --- | --- | --- |
| Movement with simultaneous drag look | Chrome emulated-touch verified | `mobile-touch-results.json`, `holdRelease`, `simultaneous` |
| Jump and creative flight | Chrome emulated-touch verified | Queued quick jump at 250 ms callbacks; native flight rise and descent |
| Placement and breaking | Authoritative server updates verified | Gold state 3830, then air state 0 |
| Multiple pointer owners and mixed keyboard/touch | Chrome emulated-touch verified | `twoOwners`, `sources` |
| Cancel and lost capture | Chrome emulated-touch verified | CDP cancel and actual DOM releasePointerCapture |
| Blur and background cleanup | DOM lifecycle boundary verified | Event dispatch while controls remain held; no real device background proof |
| Narrow, portrait, landscape, fold-like resize | Chrome emulated-touch verified | 390x844, 844x390, 1152x804, 320x568 |
| Deprecated world startup | Implemented and Chrome runtime verified | ActiveEventLoop window creation, spawn_app, asynchronous user-event handoff |
| Deprecated fixture startup | Implemented and Chrome runtime verified | Synthetic renderer shows 18 quads |
| Useful initialization failure | Browser-tested | navigator.gpu unavailable, visible enable/update/reload action |
| Wrong-resource rejection | Browser-tested | Prepared-format mismatch remains rejected |
| Canonical shared URL profile | Implemented; source reviewed | Canonical URL has no name; random per-open default; explicit names remain supported |
| Desktop multiplayer and timer regressions | Browser-tested | Existing two-client gameplay and tick drivers pass |
| Real phone, Safari, Duo GPU and fold behavior | Blocked | Parent's DEVICE_IN_USE block, no device work attempted |
| Fresh independent and cross-family reviews | Blocked, parent-owned | Claude unavailable/rate-limited, no review claimed |
| Offline browser world, native parity, X embed, public hosting | Existing unfinished scope preserved | `docs/web-port-status.md` |

## Commands and results

Every command ran from the exclusive worktree. Rust commands used:

```sh
export PATH=/Users/test/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH
export RUSTC=/Users/test/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc
python3 tools/build_web.py
cargo fmt -p horizon-web
cargo test -p horizon-web --lib
cargo clippy -p horizon-web --target wasm32-unknown-unknown -- -D warnings -A clippy::manual_range_contains
python3 tools/check_public_tree.py
git diff --check
node --check tools/web_touch_check.cjs
```

The build, four input tests, scoped Clippy, formatting, public-tree check, and diff
check pass. The aquifer `manual_range_contains` exclusion is the existing baseline
exception. No shared engine source changed. Prior native physics and renderer
suite results were read but not rerun or claimed as new phase evidence.

The browser drivers used these common environment settings:

```sh
export HORIZON_PLAYWRIGHT=/Users/test/.npm/_npx/705bc6b22212b352/node_modules/playwright
export HORIZON_WEB_BUNDLE=.local/web-owned/play-resources.zip
```

The final touch command runs actual CDP `Input.dispatchTouchEvent` input, not Rust
state mutation:

```sh
HORIZON_WEB_URL='https://tests-mac-studio.tail935cc1.ts.net:8491/?world=webproof&bridge=wss%3A%2F%2Ftests-mac-studio.tail935cc1.ts.net%3A8492' \
HORIZON_WEB_PROOF=.local/web-touch-final-4 node tools/web_touch_check.cjs
```

Other checks:

```sh
HORIZON_WEB_NAME_PREFIX=MobD HORIZON_WEB_PROOF=.local/web-mobile-desktop-2 node tools/web_play_check.cjs
HORIZON_WEB_PROOF=.local/web-mobile-tick node tools/web_tick_check.cjs
HORIZON_WEB_PROOF=.local/web-mobile-fixture HORIZON_WEB_BUNDLE= node tools/web_render_check.cjs
HORIZON_WEB_BUNDLE=.local/web-owned/wrong-version.zip HORIZON_WEB_PROOF=.local/web-mobile-wrong-2 \
HORIZON_WEB_EXPECT_ERROR="Resource manifest does not match Horizon's prepared resource format" node tools/web_render_check.cjs
```

Desktop movement reaches the other client. Both clients see placement and breaking,
and reconnect loads the edited block. Tick counts are 50 in 2501 ms normally and
51 in 2501 ms with 250 ms render callback delays. These are correctness checks,
not performance benchmarks. The synthetic fixture draws 18 quads. Resource
rejection matches the current prepared-format validation error exactly.

A separate Chrome probe removed `navigator.gpu` before page startup with
`Object.defineProperty(navigator, 'gpu', {value: undefined})`. The default fixture
then showed the visible enable-WebGPU, update-browser, and reload action.
`mobile-desktop-controls.json` also confirms hidden touch controls have zero-size hit areas in desktop Chrome.
`mobile-failure-results.json` and `.local/web-mobile-failure/no-gpu.png` preserve it.
Another probe confirmed that headless Chrome keeps background tabs `visible`.
Blur and visibility checks therefore dispatch DOM lifecycle events and are
explicitly limited to the installed cleanup boundary.

## Artifacts and limitations

`mobile-served-artifacts.json` records byte-identical HTML, JavaScript, and
WebAssembly fetched through Tailscale and compared to local `web/` files. The
wasm SHA-256 is `2afe5825e9baca8c3ba29b746c77582fd553423e8547859f1c9c2a5773955d3c`.
Generated `web/pkg` remains ignored and is not committed.

Owned-texture captures and video remain private under `.local/web-touch-final-4/`.
The four `viewport-WIDTHxHEIGHT.png` captures show actual rendered Horizon and
edge controls. `placed.png` shows the server-accepted gold block. `video/` holds
the CDP touch recording. No owned textures, screenshots, or video are committed.
Machine-readable receipts are committed under `.audit/mobile-*-results.json`.

CDP emulates DPR 2, while winit's device-pixel resize observer reports CSS-sized
buffers in this environment. The checks assert matching Window/canvas dimensions,
full viewport bounds, and the changing aspect ratio. They do not claim Retina
quality, physical safe-area cutouts, Safari WebGPU, or native fold API proof.

Early harness attempts are preserved in `.local/web-touch-proof*`. One initial
quick-jump wait did not observe the jump. Subsequent runs sample the actual height
and observe the retained quick tap. Steep placement rays could intersect the
player's placement area or aim beside the new block. The final check aims at
40 degrees, retreats, and uses a touch drag to aim at the accepted block before
breaking. An initial desktop test prefix exceeded the existing 16-character
profile limit. It was corrected to `MobD`. An initial wrong-resource test expected
an obsolete error string. The final check asserts the current error. These failed
runs are not counted as passing evidence. The final cleanup checks keep controls
held while capture is lost or lifecycle events fire. The capture check first moves
the pointer to activate pending capture. Releasing pending capture before that
move generated no lost-capture event in Chrome, as the earlier stricter probes
showed. The final receipt observes cleanup without relying on a later pointer-up.

The final browser checks close their own Chrome sessions. HTTP PID 55502, bridge
PID 88366, server PID 88504, and private bridge PID 9771 remain running. Tailscale
configuration and private resource provisioning were not changed. No PR, upstream
claim/comment, merge, public deployment, force push, or user message was made.

## Attention

Independent trail review was not run. The parent owns fresh reviews, and the
configured Claude providers remain unavailable. Actual phone validation remains
blocked until the user frees the device session or explicitly redirects the work.
