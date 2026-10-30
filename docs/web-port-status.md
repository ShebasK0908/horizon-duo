# Browser port status

The goal is real Horizon gameplay in a browser through WebAssembly and WebGPU,
with multiplayer, durable worlds, and a world URL that can be shared on X.
No playable browser build has been verified yet.

## Scope ledger

| Item | State | Acceptance evidence |
| --- | --- | --- |
| Build and rendering | Not started | Build WebAssembly and display actual Horizon world meshes through its renderer in a real browser. |
| Owned game resources | Not started | Import a matching player-owned resource bundle, validate its version, and render its models and textures. |
| Player interaction | Not started | Move in the real world and place and break blocks through existing Horizon gameplay rules. |
| Durable world | Not started | Reload and reconnect after an edit and observe that edit in the same world. |
| Offline browser world | Not started | Create, edit, save, and reopen a world without a remote game server. This follows the remote-first milestone. |
| Multiplayer | Not started | Two browser clients join one Horizon server, see each other, and observe the same block edits. |
| Shared world URL | Not started | A second browser opens the world URL and joins the intended world. |
| X embedding | Not started | Confirm the current supported integration and, if available, run the actual game inside X. A shared URL does not establish this. |
| Native regressions | Not started | Run the checks relevant to each shared engine change. |
| Review and synchronization | Not started | Inspect the real diff, obtain independent review, and push coherent tested commits to a dedicated fork branch. |

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
