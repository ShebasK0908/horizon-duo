# Codex-only candidate: shared server runtime in a browser worker

One design runner; no synthesis or cross-family review. Claude was unavailable. Inspect-only report at `3277bb98164804f8cc00bf7468493df71d6cf455`, branch `codex/horizon-web`, initially clean, upstream `fork/codex/horizon-web`, divergence `0/0` against the locally recorded remote ref. No remote refresh was performed. Repository: `/Users/test/.t3/scratch/2026-10-25-compile-this-for-the-iphone-cf98d4f8/src/horizon-web`.

## Caller usage first — proposed, not existing APIs

The player imports owned resources, chooses **Create local creative world**, builds, selects **Save and leave**, and reopens that world from the local list. Remote join links retain their present behavior.

```rust
// Creation screen: launches worker and joins its authoritative server.
let local = LocalWorlds::open(validated_resources).await?;
let play = local.create(CreativeWorld::superflat("Workshop", seed)).await?;
browser.start(play)?;

// Pause screen: success appears only after durable checkpoint and teardown.
let saved: SavedWorld = browser.save_and_leave().await?;

// World list, including after reload: loads committed server state.
let play = local.reopen(saved.id()).await?;
browser.start(play)?;
```

`LocalWorlds` hides worker setup, resource transfer, storage ownership, server login, and connection cleanup. `browser.start` consumes an opaque play connection; callers never construct packets or coordinate snapshot stages. Transport enums remain private. Repeated save/leave requests share the pending result; they cannot create competing writers.

## Feasibility verdict

**Reject a drop-in wrapper around the current integrated/dedicated server. Retain worker execution only as an extraction project with an explicit wasm gate.** Current source/build evidence does not establish a runnable browser server, and does not rule out a portable extracted runtime.

Measured: the direct native binary check, with matching rustup Cargo/RUSTC, `--locked --offline --no-default-features --target wasm32-unknown-unknown`, failed in **1.67 seconds**, before gameplay compilation: `getrandom 0.2.17` lacks `js`. Selecting the web package alongside it still failed there; the browser graph's `getrandom 0.3/wasm_js` does not solve 0.2. Temporary dependency-override experiments were inconclusive because offline caches lacked dependencies; they are not additional engine compile findings. Logs and temporary manifests accompany this report. No successful server wasm compilation is claimed.

Source blockers, separately from measured compilation:

* `main.rs:3` declares private binary module `game`; `Game` is private (`game.rs:1674`). Public-looking resource types inside that binary do not form an importable engine API. `Integrated`, `Serving`, and their entry points are `pub(super)`.
* `integrated.rs:860–914` constructs another `Game`, including `UiResources::server(Path)` and silent audio. Headless construction still loads font/resources by path. `horizon-server/src/main.rs:1` is a process supervisor, not a portable simulation library.
* `integrated.rs:390` runs a blocking clock loop; `levels.rs:353` implements the real multi-level server tick. A dedicated worker does not make `std::thread::spawn` or blocking condition variables portable.
* `lan/mod.rs:609` spawns a connection thread even when `NetConfig.listen=false`; `protocol/connection.rs:16–100` implements `Pipe` with `Mutex`/`Condvar` waits. `lan/conn.rs:464` enters blocking login/configuration/play code, including a writer thread. A worker message port is not this pipe.
* `Streaming::new` (`client-render/streaming.rs:525`) always spawns at least one worker; passing zero threads cannot produce a cooperative executor. World preparation also spawns threads (`game/world_loading.rs:176`). Compilation of meshing code does not prove this scheduler runs in wasm.
* Native `save.rs:133`, `:2074`, `:2331`, `:2388` uses file locks, region files, a writer thread and a blocking flush. `world/region.rs:53` owns `File`; IndexedDB cannot implement that interface by changing a path.
* Unconditional native-client dependencies include TLS `ureq` (`client/Cargo.toml:66`) and friends networking (`friends/Cargo.toml:16`, `socket2` and `if-addrs`). These are graph extraction targets, not errors reached by the probe.

## Existing mechanism and minimum credible milestone

The supported path creates a server-role `Game`, loads its world, sets up a hostless integrated hub, and joins its owner through memory (`integrated.rs:458–545`). That is the reuse target; the deprecated in-frame path is excluded.

The authoritative creative chain matters: `lan/hub.rs:2488` forwards `UseItemOn`; `game/lan_actions.rs:1338` validates reach/border/build height and enters `as_guest`; `Game::use_item` (`game.rs:4713`) applies actual item/block behavior. Breaking reaches `Game::break_block` (`:4530`); creative slot handling is at `lan_actions.rs:1529`. Copying only `horizon-world::level` or writing a new block setter would lose this contract.

Start with one local owner, creative superflat Overworld, ordinary place/break blocks, movement/collision, and durable reopen. Extract the existing `FlatTerrain::from_settings` (`client/flatgen.rs:73`) and its generation path (`game/world_loading.rs:409`) rather than invent terrain rules. Disable unsupported choices explicitly. This milestone does not promise mobs, all block entities, dimensions, mods, normal terrain generation or full native parity. Dependencies needed by the selected handlers still migrate; “creative” is not permission to omit their validation.

The current imported bundle is insufficient evidence for server resources: `tools/web_resources.py:27` deliberately selects a rendering/movement subset. Audit and package the exact server tables consumed by the extracted handlers. Native `Resources::WorldRest` (`game.rs:1401`) includes items, recipes and behaviors absent from the browser's equivalent. Reuse `ResourceSource`/`ResourceBundle` (`assets/source.rs:3,39`), adding byte-based loaders where only path loaders exist.

## Minimum ownership and type sketch

New **`horizon-server-core`** owns the extracted server state, guest actions, tick ordering, hub state, generation jobs and save snapshots. Move their transitive gameplay dependencies out of the native client library as needed; core must not depend on `horizon-client`, which will depend on core. Preserve old library imports by reexport where useful. This is a substantial ownership split, not exporting `Game` wholesale.

Native integrated and dedicated adapters call that same core. They retain OS threads, sockets, session locks and region writing. Browser worker owns a separate core instance, cooperative jobs and one durable world lease. Client owns only predicted motion, received chunks, input and rendering.

```rust
// New APIs; sketch only. Bodies intentionally absent.
pub struct LocalWorlds { /* worker factory, resources, world catalog */ }
pub struct LocalPlay { /* opaque connection and local save capability */ }
pub struct SavedWorld { /* world ID and committed revision */ }
impl LocalWorlds {
    pub async fn open(resources: ValidatedResources) -> Result<Self>;
    pub async fn create(&self, spec: CreativeWorld) -> Result<LocalPlay>;
    pub async fn reopen(&self, id: WorldId) -> Result<LocalPlay>;
}
// Browser.start(LocalPlay); Browser.save_and_leave() -> future<Result<SavedWorld>>.
pub struct ServerRuntime { /* private authoritative state */ }
pub struct ServerContent { /* validated, immutable server tables */ }
pub struct WorldImage { /* opaque complete committed state */ }
pub struct Checkpoint { /* immutable snapshot, private record keys */ }
pub struct CommittedRevision(u64);

impl ServerRuntime {
    pub fn open(content: ServerContent, world: WorldImage) -> Result<Self>;
    pub fn advance(&mut self, elapsed: Duration) -> Result<()>;
    pub fn checkpoint(&mut self) -> Result<Checkpoint>;
}
// Internal: incremental packet session + bounded queues; shared native/browser.
// receive(bytes), poll_output(), service_jobs(budget), attach_owner(profile).
// Persistence adapter: load(id), acquire(id), commit(checkpoint), release().
```

`advance` preserves server tick order while yielding between worker tasks; it must not become an unbounded loop. Extract the blocking server connection state machine into incremental protocol handling, retaining memory/offline identity and no-compression policy (`lan/conn.rs:718,778`). Native connection threads drive the same state machine. Worker adapters use ordered, bounded transferable-byte queues with close/error/backpressure handling, never `Pipe` waits.

`BrowserClient` currently embeds `ClientConnection` (`web/client.rs:60`), but already drives `ClientSession::receive` incrementally (`:930`). Replace the concrete connection privately with remote-WebSocket/local-worker variants; keep packet application, physics and renderer unchanged. Local bytes traverse real login/configuration/play and authoritative block updates, not an echo.

## Persistence boundary and dataflow

Input → existing client protocol → worker memory queue → shared server session/hub → guest action/tick → authoritative packets → existing client chunk decoder/mesher/renderer.

At a tick boundary, checkpoint creation freezes an immutable image, including generator configuration/seed, chunks and scheduled ticks, owner state, entities/POI and metadata actually supported by the milestone. Extract existing NBT encoders/decoders (`save.rs:1168,1421`) and level/player serialization from file I/O. Audit all writes under `Game::save_game_metadata` (`game.rs:12640`); chunk-only persistence is insufficient.

Use IndexedDB versioned records with dimension/chunk keys plus metadata and revision. Commit changed records and the manifest pointer atomically in one transaction; persist before acknowledging `SavedWorld`. A failed/quota-aborted transaction leaves the preceding revision reopenable and the current session visibly unsaved. Single-worker ownership plus a cross-tab exclusive lease prevents concurrent world writers; stale revision checks add protection. Reopening validates schema, game data version and resource identity before interpreting state IDs. Do not rely on unload callbacks. Native adapter continues existing region/NBT storage; browser storage need not pretend to be `.mca` files.

## Verifiable sequence and scope ledger

1. **Grounding/probe: inspected; native wasm build blocked.** Preserve logs, pin ownership and establish the new core's dependency closure before port implementation.
2. **Shared extraction: designed, unimplemented.** Move supported server runtime/actions and adapt native integrated/dedicated callers first. Run targeted server gameplay, dimension-player, save-roundtrip and LAN checks with scratch data; no new in-frame tests.
3. **Browser scheduler/session/content: designed, unimplemented.** Compile the actual worker entry with `--locked`; exercise real native adapters against the extracted session, then real browser owner login and generated chunks. Measure worker memory/tick responsiveness before expanding scope.
4. **Persistence: designed, unimplemented.** Test commit/reopen, interrupted commit, quota failure, mismatched resources, and competing tabs.
5. **First browser acceptance: unverified.** Serve static app/wasm over HTTP; forbid all WebSocket constructors and game/bridge traffic. Create superflat, move/collide, place and break actual blocks, verify authoritative acceptance/rejection, save, terminate worker/reload page, reopen matching edits and player state. Repeat interrupted-save recovery. Inspect captures and storage receipts.
6. **Remote/native regressions: not rerun here.** Repeat existing two-browser remote convergence/reconnect/share-link checks and relevant native checks after extraction. Local persistence does not make a local world URL shareable; existing remote multiplayer remains the sharing path.

We accept extraction cost for one authoritative implementation. A separate miniature engine is rejected because it exposes divergent gameplay rules. A full native binary transplant hides none of the platform dependencies from implementers. Shared-memory wasm threads are not assumed; they would still leave filesystem, ownership and content barriers.

Open gates: how large is the selected handlers' dependency closure, and can the extracted runtime meet browser memory/tick budgets? Neither is settled by this probe. No service worker or full-app offline caching is required: **no remote game server** still permits HTTP delivery of the app. Synthesis and implementation belong to the coordinator. No repository edits, game launches, devices or external actions occurred.

Final working-tree audit: HEAD/branch/upstream divergence are unchanged, but `crates/horizon-web/src/input.rs` became modified during inspection. This runner made no repository edits; that concurrent change was left untouched. Server/core pointers refer to the pinned HEAD; no acceptance claim covers the changed input file.
