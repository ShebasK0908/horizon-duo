# Browser port status

The goal is real Horizon gameplay in a browser through WebAssembly and WebGPU,
with multiplayer, durable worlds, and a world URL that can be shared on X.
Browser rendering and owned-resource import have been verified in Chrome.
Creative movement, block placement and breaking, two-client convergence, and
server-backed persistence have now been verified locally in Chrome. Full native
client parity and in-X execution remain unfinished. The user explicitly deferred
offline browser worlds to focus on multiplayer and X integration.

## Scope ledger

| Item | State | Acceptance evidence |
| --- | --- | --- |
| Build and rendering | End-to-end verified on local Chrome WebGPU | Build WebAssembly and display actual Horizon world meshes through its renderer in a real browser. |
| Owned game resources | End-to-end verified for local import | Import a matching player-owned resource bundle, validate its version, and render its models and textures. |
| Player interaction | End-to-end verified for creative movement, placement and breaking | Move in the real world and place and break blocks through existing Horizon gameplay rules. |
| Durable world | End-to-end verified for server-backed reconnect and process restart | Reload and reconnect after an edit and observe that edit in the same world. |
| Offline browser world | Explicitly deferred by the user; design inspection complete, implementation not started | Future acceptance remains create, edit, save, and reopen without a remote game server. Excluded from the current multiplayer and X milestone. |
| Multiplayer | End-to-end verified for two local browser clients | Two browser clients join one Horizon server, see each other, and observe the same block edits. |
| Shared world URL | End-to-end verified locally and through private Tailscale HTTPS/WSS in Chrome | A second browser opens the world URL and joins the intended world. |
| X embedding | In progress; private Player Card candidate and local cross-origin iframe tested; X remains unverified | Confirm the current supported integration and, if available, run the actual game inside X. A shared URL does not establish this. |
| Native regressions | Locally tested for current shared changes | Run the checks relevant to each shared engine change. |
| Touch controls and responsive viewport | End-to-end verified in desktop Chrome with CDP emulated touch | Chrome GPU with emulated touch must verify movement, simultaneous look, actions, cleanup, and live viewport changes. |
| Browser application lifecycle | Implemented and browser-tested in Chrome | Both startup callers use active-event-loop window creation and asynchronous renderer startup without deprecated APIs. |
| Phone and Duo runtime | Blocked | Device session is owned by another session; no phone, Safari, or actual fold proof is claimed. |
| Review and synchronization | Pushed checkpoints; Codex runtime and card reviews pass; BLOCKED: cross-family independent review because Claude is unavailable | Inspect the real diff, obtain independent review, and push coherent tested commits to a dedicated fork branch. |

Compilation, a blank canvas, a triangle, a network echo, and a different voxel
game do not satisfy the gameplay requirements. A partial checkpoint keeps every
unfinished item in this ledger.

## Architecture decision

The first milestone is a remote browser client. It reuses Horizon's renderer,
mesher, chunk data, physics, and protocol. The existing native server owns
authoritative gameplay and world persistence. Offline browser worlds are
explicitly deferred by the user.

The alternative is to port the full native client and integrated server at once.
That requires replacing platform assumptions throughout startup, resources,
workers, sockets, and saves before the first playable build. The remote-first
order establishes browser rendering and authoritative multiplayer earlier.

The implementation starts with these ownership boundaries, refined against the
actual code before changing APIs.

- `BrowserClient` owns its session, rendered world, renderer, and input.
- A session state enum owns connection, login, configuration, play, and closure.
  The session also owns incremental framing, compression, and encryption.
- Client world state reuses existing remote packet application and prediction.
  A second implementation of gameplay rules is not an acceptable shortcut.
- A resource source exposes immutable bytes and directory listings. Native
  resources use files. Browser resources use a validated imported bundle.
- A bounded WebSocket bridge has configured upstream servers and allowed
  origins. It is not an unrestricted TCP proxy. WebSocket messages can contain
  part of a Minecraft packet or several packets.

## Verification order

1. Record the unmodified WebAssembly build failure.
2. Make the existing protocol and rendering dependency graph compile for the
   browser while preserving native defaults.
3. Run the actual Horizon renderer with an original synthetic mesh fixture.
4. Import owned resources and receive actual remote world chunks.
5. Verify movement, placement, and breaking through the existing server.
6. Verify two clients, reconnect persistence, and the shared world URL.
7. Verify the supported X integration and report any external limitation.
8. Explicitly deferred by the user. Add offline browser execution and browser
   persistence, then repeat gameplay checks without a server in a future milestone.

The public fixture must contain no Mojang files. Game runs use fresh scratch data
directories as required by `AGENTS.md`. Evidence records distinguish source
review, compilation, automated tests, and browser runtime checks.

## Baseline and coordination

The dedicated branch starts from fetched upstream commit
`a12b5151e139b346aef09c3ceacb4e5f925694a8`. The original iPhone checkout is
untouched. The earlier baseline at `771394b76` failed compiling `getrandom`
0.2.17 because its browser `js` feature was not enabled. Repeat the check after
the base update.

The matching compiler and Cargo must both come from rustup. Homebrew binaries
otherwise select a different sysroot and incorrectly report that the installed
WebAssembly standard library is missing.

```sh
RUSTC=/Users/test/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc \
  /Users/test/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo \
  check -p horizon-client --bin horizon-client --no-default-features \
  --target wasm32-unknown-unknown
```

[Upstream issue #4153](https://github.com/horizon-rust/horizon/issues/4153)
has another contributor's claim, with no linked implementation found during
the initial investigation. This work is the user's fork-local port. It does
not claim that issue or open a competing upstream pull request.

Claude research tasks failed with rate limits. Their absence is not an
independent review pass. Implementation and review status stay separate.

## Implementation checkpoint

The first unit enables WebGPU on the existing renderer dependency graph.
Its test is an original synthetic `SectionMesh` rendered by `Renderer` in a browser.
This unit does not establish gameplay, imported resources, or multiplayer.

## Run the rendering checks

Install the WebAssembly target with `rustup target add wasm32-unknown-unknown`.
Install `wasm-bindgen-cli` version 0.2.129, matching `Cargo.lock`, into `.local/web-tools`.
Run `python3 tools/build_web.py`, then serve `web/` on localhost.
The default page draws an original synthetic mesh through Horizon's `Renderer`.
The page is renderer validation and has no gameplay session.

Package your own prepared resources with
`python3 tools/web_resources.py PATH_TO_PREPARED_RESOURCES .local/browser-resources.zip`.
Open `/?owned` and import the archive to check real model decoding and textures.
Resource files stay on the player's computer. The import checks the preparation
schema, Minecraft version, and data version before reading the block registry.
The browser input clipboard currently retains text inside the app only.
Native file loading, native clipboard integration, Vulkan HDR, and the native
allocator remain available on native targets.

Run `HORIZON_PLAYWRIGHT=PATH_TO_PLAYWRIGHT node tools/web_render_check.cjs` against
`http://127.0.0.1:8766`. Set `HORIZON_WEB_BUNDLE` to test the owned import and
`HORIZON_WEB_PROOF` to a fresh local output directory. Set
`HORIZON_WEB_EXPECT_ERROR` to check an expected import failure.
The check requires rendered frames, nonempty mesh geometry, and no browser
shader-validation errors. The capture is inspected separately.

The rendering checkpoint passed 51 input tests, 124 protocol tests, 4 thread
helper tests, 11 asset tests, and 627 renderer tests. Sixteen renderer tests were
ignored by their existing definitions. Strict Clippy encounters the unchanged
`manual_range_contains` lint in `horizon-world/src/worldgen/aquifer/noise_based.rs:466`.
The selected libraries pass with that one lint suppressed. Full workspace and
vanilla parity verification remain outstanding.

The public synthetic capture and browser result are in
[the rendering evidence](../.audit/web-render-browser.json) and
[the capture](../.audit/web-render-browser.png). The owned-model result is in
[the owned-resource evidence](../.audit/web-owned-browser.json).
A [wrong-version import](../.audit/web-wrong-version-browser.json) was rejected in Chrome.
Owned textures and their captures remain under ignored `.local/` paths.

## X integration evidence

The old official Player Card documentation URL currently redirects to
[X's general developer overview](https://docs.x.com/overview).
It does not establish current game-embedding support.
[Rosebud's October 10, 2026 engineering report](https://lab.rosebud.ai/engineering/making-rosebud-games-playable-in-x-posts)
describes real games in Player Card iframes, HTTPS player URLs, framing headers,
and client-dependent inline playback. This is first-party implementation evidence
for Rosebud, not a verified Horizon integration or a current X support contract.
Horizon still needs a hosted HTTPS build and an authorized real-post check.
No deployment or post was performed. Share links and in-X play remain separate.

`tools/web_embed_check.cjs` now runs actual Horizon in a 720x720 cross-origin
iframe under a local HTTP parent. Its sandbox permits scripts, the game's origin,
pointer lock, and forms. Keyboard input moved the player in the iframe. The
isolated native server acknowledged gold placement. The parent inspected the
capture and saw the placed block in the
framed world. This proves one Chrome iframe configuration. It does not establish
X's actual sandbox, crawler access, Player Card acceptance, or inline execution.
The [parent verification report](../.audit/mobile-parent-verification.md) records
the command, failed setup attempts, and remaining limits.

`tools/web_card.py` generates a static Player Card candidate for an explicit
HTTPS game URL and preview URL. Its ignored output is currently available at
the private Tailscale `/share.html` route. The original SVG and PNG preview use
no imported game resources. Private HTTP checks read the metadata with both
Twitterbot and ordinary user agents. A Chrome check renders the share page and
follows **Play Horizon** to the game's resource import. This is private candidate
validation, not a real X crawler request. A fresh Codex review found no verified
issues in the generator, URL boundary, decoded tags, preview delivery or play-link
navigation. [Its results](../.audit/x-card-review.md) remain private-candidate
evidence. [The card preparation guide](web-x-card.md)
records generation and public-validation steps. Public hosting, actual X card
acceptance, and in-X gameplay remain unfinished.

## Remote-world checkpoint

The browser now joins an actual native Horizon dedicated server through a bounded
WebSocket bridge and renders server chunks. `ChunkFormat` moved from the private
`Game` module into `horizon-client-render::remote_chunks`. Both clients use that
same decoder. The browser uses the existing `Mesher`, `World`, and `Renderer`.
This checkpoint established a stationary world client; the gameplay checkpoint
below adds input and peer rendering.

`horizon-protocol::client_session` owns incremental framing, compression, optional
offline encryption, login, configuration, and play. Online account authentication,
required server packs, code-of-conduct prompts, and reconfiguration are rejected
with explicit errors. They are not silently accepted or treated as supported.
The dedicated test server uses offline profiles on loopback.

Run the native binary from a fresh test environment with
`python3 tools/run_web_test_server.py --resources PATH_TO_PREPARED_RESOURCES`.
The tool prints its private test directory and log path. Type `stop` to end it.
Start `horizon-web-bridge --world webproof --upstream 127.0.0.1:25575 --origin http://127.0.0.1:8766`.
The bridge serves only that configured upstream and world path. It requires an
exact allowed origin, caps concurrent connections, bounds WebSocket messages,
and applies backpressure separately in each direction.

Open `http://127.0.0.1:8766/?world=webproof&bridge=ws://127.0.0.1:8767&name=BrowserOne`
and import a fresh archive made by `tools/web_resources.py`.
`tools/web_world_check.cjs` automates that stationary world check.
[The browser result](../.audit/web-world-browser.json) records nine received columns
and a drawn resident section. The capture remains private under
`.local/web-world-proof-2/world.png`. Four invalid world/origin/query handshakes
were [rejected with HTTP 403](../.audit/bridge-origin-check.json).

The native client compiles, its two remote chunk-decoder tests pass, and the
shared protocol and renderer library suites pass. Two new session tests cover
fragmented/coalesced frames across compression and state changes, plus malformed
frame closure. The following checkpoint verifies movement, block interactions, peers, and server
persistence. Offline worlds remain unfinished.

## Browser gameplay checkpoint

`horizon-client-physics` now owns the existing player motion, fluid, sprint,
pose, hunger, health, and landing bindings. Native `horizon-client` reexports
those modules under their old paths. A small `SavedMotion` value separates
player loading from the native world's file representation; the native save
round-trip test still runs against `Level`. This extraction avoids maintaining
a second set of browser movement rules.

The browser feeds keyboard input into the same `MotionWorld` simulation and
uses the existing `PositionSender` to send motion to the authoritative server.
The shared block ray picker selects actual outline shapes. Creative placement
uses `SetCreativeModeSlot` and `UseItemOn`; breaking uses `PlayerAction`. The
browser waits for authoritative block updates. The server retains the original
placement, collision, and destruction rules.

Remote players use the existing prepared player model and model renderer,
textured with the player's imported default skin. Network motion uses the
existing stepped interpolation and quantized relative-position codecs. Custom
skins, animated poses, inventory screens, survival mining progression, item
entities, mobs, audiovisual events, and full native UI parity are not yet
implemented in the browser shell. These limits matter even though the verified
creative loop is playable.

[The two-client browser result](../.audit/web-play-browser.json) records a peer
movement observed by the other browser, 288 prepared model vertices, placement
of state 3830 at `[0,-60,2]` in both clients, reconnect loading the same block,
and the break update reaching both clients. The original-resource captures
remain private under `.local/web-play-proof-2/`; the inspected `peer.png` shows
the imported player model in the actual server world.

A separate disposable server on port 25576 was stopped cleanly and restarted
against the same world. The browser read the diamond block state 6865 at
`[0,-60,2]` from the restarted server's chunks. See the
[write result](../.audit/web-persistence-write.json),
[read result](../.audit/web-persistence-read.json), and
[server receipt](../.audit/web-persistence-server.txt).
This proves server-backed persistence, not an offline browser save.

Run `HORIZON_PLAYWRIGHT=PATH_TO_PLAYWRIGHT node tools/web_play_check.cjs` with the
HTTP server, configured bridge, and native test server running. Set
`HORIZON_WEB_URL`, `HORIZON_WEB_BUNDLE`, and a fresh `HORIZON_WEB_PROOF` directory
as needed. For persistence, start a separate server with `--port 25576`, bridge
8768 to it, run `tools/web_persistence_check.cjs`, type `stop` in that server,
resume its printed test directory with `--resume`, and run the same check with
`HORIZON_WEB_PERSIST_MODE=read`. Each server start creates fresh environment homes.

The shared suites passed 39 physics tests with the actual staged resources,
627 renderer tests (16 existing ignored tests), and 126 protocol tests. The
native saved-attribute round trip passes with the resources present. Native mob
model and related tests pass 187 cases (6 existing ignored tests). The native
client binary checks successfully. Scoped WebAssembly Clippy passes with the
previously recorded unchanged aquifer lint suppressed. These checks do not
constitute full workspace or vanilla parity verification.

## Private Tailscale runtime

The parent configured and verified a tailnet-only runtime at
`https://tests-mac-studio.tail935cc1.ts.net:8491/?world=webproof&bridge=wss%3A%2F%2Ftests-mac-studio.tail935cc1.ts.net%3A8492`.
Port 8491 proxies the existing HTTP server on 8766. Port 8492 proxies a separate
bridge on 18767, restricted to world `webproof`, upstream 25575, and the exact
origin `https://tests-mac-studio.tail935cc1.ts.net:8491`. The original bridge on
8767 is unchanged. Neither new Tailscale port uses Funnel.

The parent reports HTTP 200 for the page and WebAssembly, WSS 101 for the
allowed origin, 403 for a wrong origin, and matching SHA-256 for the private
`/resources.zip` download. The download proxies 18766 through a temporary
symlink to `.local/web-owned/play-resources.zip`, so regenerated bundles update
that download. This is parent-supplied runtime evidence; the implementation
checks above used local Chrome. Touch input is now checked in desktop Chrome with emulated touch, as described below. Mobile WebGPU has not been verified. The native world and these runtime services remain running for user
testing. This is not a public HTTPS deployment or an X embed test.

The implementation owner then ran the actual world check through that exact
Tailscale HTTPS/WSS route in Chrome, using a separate `TailWebProof` profile.
[The browser receipt](../.audit/web-tailnet-browser.json) records 45 received
columns, two drawn sections, and player movement from `[0.5,-60,0.5]` to
`[0.5,-60,1.6003]`, with no page or shader validation errors. This checks the
current compiled build and the private route together. It does not establish
phone WebGPU or in-X execution.

The playable creative checkpoint is `84f20823f` on `codex/horizon-web`, pushed
to `fork/codex/horizon-web`. Parent architecture synthesis and independent review
remain outstanding. No PR, merge, public deployment, release, or X post was made.
The temporary persistence-test server and bridge were stopped after their checks.
The user-facing HTTP server (PID 55502, 8766), original bridge (PID 88366, 8767),
and native world (PID 88504, 25575) remain running. Parent-owned ports 8491,
8492, 18766, and 18767 were left untouched.

## Parent review checkpoint

The parent reran two-client gameplay through the Tailscale HTTPS/WSS address.
[The new receipt](../.audit/parent-play-browser.json) records movement observed
by the other player, a placed block observed by both clients, reconnect to that
edit, and breaking observed by both clients. The rendered capture was inspected
locally. This run reports no browser or shader errors.

The shader review found a suppressed WebGPU diagnostic. Removing the suppression
made Chrome reject implicit glint sampling after nonuniform control flow, while
the native Naga validator still passed. The shader now computes gradients before
discard and uses explicit-gradient sampling. The browser compiler reports no
diagnostics, and all 19 selected native graphics tests pass, including HLSL
translation. The rebuilt game passed the two-client Tailscale check.

The gameplay check now waits for the importer to become visible before selecting
resources. Its previous cold-start race could select the hidden file input before
WebAssembly startup installed the event handler. A nearby-target assertion also
replaces the fixed aiming delay.

Comment review removed ten narrative lines and two suppressions, including the
shader suppression and a redundant deprecation allowance. Two function-level
deprecation allowances remained at that checkpoint. The mobile phase below
migrates both browser startup callers to the current winit application lifecycle.
Claude's independent review attempt failed with a rate limit.
Neither the parent check nor the remaining Codex review is a cross-family verdict.

## Runtime review findings

The first correctness review reproduced four browser bugs. All four remain
required fixes. [The report](../.audit/adversarial-round1.md) preserves its exact
commands, observations, and artifact provenance.

| Finding | State | Required verification |
| --- | --- | --- |
| R1. Authoritative attributes | End-to-end verified in Chrome | A saved movement-speed update reaches player physics, including the initial synchronization path. Updates received before player construction are retained. |
| R2. Game-mode changes | End-to-end verified in Chrome | Creative mode collides with a test wall, then server-selected spectator mode crosses it. |
| R3. Creative selection after inventory edits | End-to-end verified in Chrome | Clearing the inventory does not require changing the visible material before another successful placement. |
| R4. Rendering-dependent tick rate | End-to-end verified in Chrome | Delayed render callbacks preserve elapsed fixed simulation ticks and comparable input travel. |

The fixes reuse native attribute synchronization and the native tick timer.
Creative placement supplies the selected stack each time. The selected-item
cache is removed. The server remains authoritative over accepted edits.

[The four assertions](../.audit/review-state-results.json) compare actual browser
observations from the reviewer's isolated native-server fixtures. Saved movement
speed values `0.5` and `0.1` now produce distinct travel. Creative movement stops
at the wall while spectator movement crosses it. Gold placement after an inventory
clear produces state `3830` without changing the selection. The delayed-frame
probe observes comparable simulation ticks and travel. These are correctness
checks with synthetic inputs, not performance benchmarks.

The new `tools/web_tick_check.cjs` checks actual outgoing `ClientTickEnd` frames
against elapsed time at normal rendering cadence and with 250 ms callback delays.
Run it against the default compression-enabled test server:

```sh
HORIZON_PLAYWRIGHT=PATH_TO_PLAYWRIGHT node tools/web_tick_check.cjs
```

`HORIZON_WEB_URL`, `HORIZON_WEB_BUNDLE`, and `HORIZON_WEB_PROOF` select the world,
owned resources, and output directory. [The tick receipt](../.audit/review-tick-check.txt)
records both observation windows. Long stalls retain the native client's cap of
ten catch-up ticks per frame.

All 40 shared physics tests ran with an absolute owned-resource path. None reported
missing resources. The eight native clock tests passed after moving `Timer` into
the shared crate. The native binary compiles, the WebAssembly build passes, and
scoped WebAssembly Clippy passes with the same documented baseline lint excluded.
The fix diff still needs a fresh review. Cross-family review remains blocked.

## Fresh state-fix verification

A fresh verifier reran the served artifact at `6ee463b8d` and independently
asserted R1 through R4. [The report](../.audit/fresh-state-verification.md) and
[results](../.audit/fresh-state-results.json) identify the executed commands,
packet observations, and limitations. Separately aimed captures were inspected
and show textured peers in both directions. The verifier also confirmed that
the served files match the local artifacts. Native suites were not rerun in that
verification; those remain the parent's checks. Cross-family review remains
blocked by Claude rate limits.

## Touch input and browser lifecycle checkpoint

The browser now has 48 CSS-pixel edge controls for movement, jump, descent,
placement, and breaking. Drag the world with a second finger to look while
moving. Double-tap Jump to toggle the existing native creative flight behavior.
Hold Jump to rise and Down to descend. Keyboard and touch holds have separate
ownership and merge into the existing `Keys` input. Pointer ownership, retained
jump transitions, and queued actions live in Rust. A release from one source
does not release the other source. Cancel, lost capture, blur, visibility changes,
and dimension resets clear the relevant state.

The page uses viewport metadata, dynamic viewport height, and safe-area insets.
Controls stay at the outer edges and retain their CSS size across resizing.
Winit's canvas resize observer updates Window dimensions, and both callers pass
those dimensions to `Renderer::resize`. The renderer uses its current dimensions
for the camera aspect ratio. This is ordinary web resizing. No native hinge,
reserved-region, or occlusion API is available or claimed here.

`application.rs` creates each window through `ActiveEventLoop` in `resumed`,
starts the renderer asynchronously, and receives the result through a user event.
Both callers use `spawn_app`. The two broad deprecation allowances are removed.
Graphics startup failures show a reload and browser-update action. Module and
WebAssembly loading failures also appear visibly. Wrong-resource imports still
reject. The canonical shared URL above omits a fixed profile name. Explicit
`name` query parameters remain supported for tests and persistent test profiles.

[The mobile phase report](../.audit/mobile-verification.md) records exact commands,
checks, and private evidence paths. [The touch receipt](../.audit/mobile-touch-results.json)
passed through private Tailscale HTTPS/WSS in desktop Chrome with CDP touch input.
It covers press, hold, release, simultaneous look, multiple owners, cancellation,
lost capture, keyboard-plus-touch ownership, retained jump taps at 250 ms render
callback delays, creative flight, and authoritative gold placement and removal.
Blur and visibility cleanup were exercised by DOM lifecycle event dispatch,
not a real phone background transition. Viewport changes to 390x844, 844x390,
1152x804, and 320x568 updated actual canvas and Window sizes and kept every
button inside the viewport. Captures were visually inspected. Emulated DPR was
2, but winit's device-pixel resize observer reported CSS-pixel-sized drawing
buffers under CDP. This proves aspect and viewport adaptation, not Retina or
mobile GPU fidelity.

The existing desktop two-client gameplay check and fixed-tick check pass.
The synthetic fixture, four input unit tests, scoped WebAssembly Clippy, and
wrong-resource rejection pass. No shared engine source changed in this phase,
so the earlier native engine test results remain prior evidence. Actual phone,
Safari, and Duo execution remain blocked by `DEVICE_IN_USE` from the parent's
device session. This phase did not drive any device. Independent review remains
blocked by unavailable Claude providers and belongs to the parent. Offline
browser worlds, full native parity, X embedding, and public deployment retain
their existing unfinished status.

## Mobile input corrections

The Codex adversarial mobile review proved two P2 input bugs after the initial
mobile checkpoint. A stalled frame could replay widely separated Jump presses
as a double tap. Deferred blur cleanup could erase a new touch or key press after
focus returned. The parent reproduced both against the existing artifact.

Jump transitions now retain their event times and share ownership between the
keyboard and touch pointers. A new press after the 350 ms flight gesture window
starts a new sequence in the existing motion kernel. Blur and visibility handlers
clear keyboard and pointer state at the boundary. The redraw reset flag is gone.

The expanded touch driver verifies separate taps during stalled rendering and
fresh touch and W input after trusted Chrome blur and focus events. It also
preserves the existing movement, valid double-tap flight, block action, and resize
checks. Six input unit tests, the WebAssembly build, scoped Clippy, the expanded
touch driver, desktop multiplayer, and the fixed-tick check pass. No shared native
engine source changed. [The first fix-round report](../.audit/mobile-input-fix-round1.md)
preserves commands and before/after evidence. Fresh Codex review verified both
original corrections and found one additional cancellation defect.
Cross-family review, actual phone evidence, offline worlds, native parity, and X
execution retain their existing unfinished status.

Cancelling a pending Jump touch could clear an earlier completed keyboard jump
from the shared queue. The parent reproduced this failure. Transitions now retain
their source until sampling, and cancellation removes only the active pointer's
unconsumed press. Earlier keyboard and pointer gestures survive, including reused
pointer IDs. Nine input tests, the WebAssembly build, scoped Clippy, all six focused
browser probes, and the expanded full touch driver pass. Matching server-delivered
observer ascent confirms the corrected keyboard jump. [The second fix-round
report](../.audit/mobile-input-fix-round2.md) preserves evidence and commands.
Fresh Codex review passed the latest correction with no additional runtime-proven
finding. It passed 31 primary browser probes, the full committed touch driver,
and nine input unit tests. [The fresh review](../.audit/mobile-final-runtime-review.md)
records exact scope and limits. A supplementary reused-ID Look capture-loss probe
could not run because Chrome rejected the harness's capture request; it is not
counted as passing runtime coverage. Physical phone/Safari/Duo and cross-family
review remain unverified.

## Offline architecture investigation

Two Codex-only candidates inspected the actual server and shared-kernel paths.
Neither candidate implemented a browser world. Both reject a browser-specific
imitation of native gameplay and the deprecated native in-frame path.

[The supported-server candidate](../.audit/offline-design-server.md) found that
a worker wrapper is insufficient. Supported server state, incremental sessions,
cooperative jobs, byte-based content and durable snapshots require extraction.
Its direct native-binary Wasm check failed first at getrandom 0.2's missing
browser feature. Later platform and visibility barriers are source findings,
not compiler errors reached by that probe.

[The shared-kernel candidate](../.audit/offline-design-kernel.md) proposes a smaller
superflat building milestone using extracted native action policy, FlatTerrain,
level behavior and save codecs. A temporary probe with source-identical
FlatTerrain and existing public APIs passed a pinned Wasm type check.
[The probe receipt](../.audit/offline-kernel-probe.json) distinguishes that result
from initialized gameplay or browser durability, which remain unverified.
Admitted actions must preserve native obstruction, connections and effect
ordering. Unsupported behavior cannot be silently dropped.

Both designs require real native-code extraction, byte-based resources and
atomic browser persistence. The user answered the scope question with
"Defer offline; focus on multiplayer and X". Offline implementation is explicitly
deferred and remains unstarted. The designs and original create/edit/save/reopen
acceptance item stay in this ledger for a future milestone.

## Isolated single-origin private staging

A separate `xproof` demo is staged privately on 8493, using a new disposable
native world and a seven-file browser package. HTTP and WebSocket traffic share
one TLS origin. The two-client Chrome check passed movement, placement,
reconnect and removal. Wrong origins/worlds receive 403; the correct WebSocket
upgrade receives 101. `/resources.zip` is absent and returns 404.

All existing Serve and Funnel routes and the original private world are
unchanged. Port 8493 is not public. [The staging report](../.audit/public-candidate-preparation.md)
records package hashes, commands, process ownership and remaining public/X
validation. It is preparation for an approved public endpoint, not a public
deployment or X execution claim.

## Current review reconciliation

The latest runtime reviewer pinned `4ad4970aa436f2783b8f53ff2dd81645230b86ac`
and reviewed input correction `3277bb981..d4317cd74`. The parent subsequently
verified that all Horizon web input/client source and the touch harness remain
identical through `2dfdbee7e`. Later changes record review and private staging
evidence only. The review does not retroactively verify those operational
changes. The parent separately ran the two-client check through staged 8493.

No runtime-proven mobile finding remains open. This does not establish exhaustive
input coverage, actual phone execution, full native parity, or a cross-family
verdict. Claude providers remain unavailable, so shipping is still blocked by
the invoked skill's independent-review gate. The original private multiplayer
build and isolated private staging demo remain live. Public hosting and actual
X execution have not occurred. Offline implementation remains unstarted and is
explicitly deferred by the user. The active milestone is multiplayer and X
integration. Public hosting authorization and the independent-review gate remain
pending; the offline scope answer does not resolve either.
