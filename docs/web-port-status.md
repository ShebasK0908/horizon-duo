# Browser port status

The goal is real Horizon gameplay in a browser through WebAssembly and WebGPU,
with multiplayer, durable worlds, and a world URL that can be shared on X.
Browser rendering and owned-resource import have been verified in Chrome.
No playable browser build has been verified yet.

## Scope ledger

| Item | State | Acceptance evidence |
| --- | --- | --- |
| Build and rendering | End-to-end verified on local Chrome WebGPU | Build WebAssembly and display actual Horizon world meshes through its renderer in a real browser. |
| Owned game resources | End-to-end verified for local import | Import a matching player-owned resource bundle, validate its version, and render its models and textures. |
| Player interaction | Not started | Move in the real world and place and break blocks through existing Horizon gameplay rules. |
| Durable world | Not started | Reload and reconnect after an edit and observe that edit in the same world. |
| Offline browser world | Not started | Create, edit, save, and reopen a world without a remote game server. This follows the remote-first milestone. |
| Multiplayer | In progress; stationary server session only | Two browser clients join one Horizon server, see each other, and observe the same block edits. |
| Shared world URL | In progress; query routes to a configured world | A second browser opens the world URL and joins the intended world. |
| X embedding | In progress; source research only | Confirm the current supported integration and, if available, run the actual game inside X. A shared URL does not establish this. |
| Native regressions | Locally tested for current shared changes | Run the checks relevant to each shared engine change. |
| Review and synchronization | In progress; checkpoint pushed, parent review outstanding | Inspect the real diff, obtain independent review, and push coherent tested commits to a dedicated fork branch. |

Compilation, a blank canvas, a triangle, a network echo, and a different voxel
game do not satisfy the gameplay requirements. A partial checkpoint keeps every
unfinished item in this ledger.

## Architecture decision

The first milestone is a remote browser client. It reuses Horizon's renderer,
mesher, chunk data, physics, and protocol. The existing native server owns
authoritative gameplay and world persistence. Offline browser worlds remain a
separate unfinished item.

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
8. Add offline browser execution and browser persistence, then repeat gameplay
   checks without a server.

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

## Remote-world checkpoint

The browser now joins an actual native Horizon dedicated server through a bounded
WebSocket bridge and renders server chunks. `ChunkFormat` moved from the private
`Game` module into `horizon-client-render::remote_chunks`. Both clients use that
same decoder. The browser uses the existing `Mesher`, `World`, and `Renderer`.
This is a stationary world client. It has no player movement or block input yet.

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
frame closure. Movement, authoritative block interactions, peer rendering,
reconnection after edits, and offline worlds remain unfinished.
