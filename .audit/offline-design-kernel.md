# Codex-only independent design candidate

## Usage first

Proposed caller API (not existing exports):

```rust
// Create: the browser store supplies an exclusive world lease.
let mut local = OfflineWorld::create(resources, NewWorld::superflat(seed), lease).await?;
// A fixed simulation tick: input has domain actions, never network packets.
local.advance(input)?;
view.draw(local.scene(), local.player())?;
// Explicit Save and close: success means the storage transaction committed.
let saved = local.save().await?;
drop(local);
let reopened = OfflineWorld::open(resources, saved.world_id(), store).await?;
```

The offline constructor must never construct `ClientConnection` or `ClientSession`. Existing remote entry points remain available with their current protocol authority.

## Problem and verdict

A small offline creative runtime is credible **as a bounded superflat building milestone after shared extraction**, not as a drop-in assembly of today's APIs. Reject a browser-only imitation of `Game::use_hand`/`break_block`, silent side-effect loss, or use of deprecated `--in-frame`.

First expose a positive building palette of ordinary solids, stairs, slabs and fences with plain item stacks. These exercise real orientation, slab merging, connections, obstruction and neighbour updates. Containers, falling blocks, fluids, growth, entities, portals and arbitrary item components require their native consumers before admission. This is a proposed milestone boundary, not completion of full native creative parity or permission to discard the preserved offline-world requirement.

I choose existing superflat generation over first porting procedural generation because it removes filesystem/thread startup and decoration dependencies while still creating actual Horizon chunks through Horizon generation code.

## Current mechanism and reusable APIs

All pointers below are relative to the pinned repository.

* `crates/horizon-web/src/client.rs:54,669,814,849`: BrowserClient owns a rendered `World`, motion, input and mandatory network state. Movement calls `MotionWorld::controlled_simulation_tick`; interaction sends creative-slot/use/dig packets. It does not execute authoritative creative actions locally.
* `crates/horizon-client/src/game/lan_actions.rs:1216,1348,1402`: real server creative START breaks immediately through `as_guest -> break_block`; use validates reach, cursor, height, protection and border, then invokes `use_item`. The integrated server is a second native `Game`, not a portable library (`game/integrated.rs:1,53,74`). `game/levels.rs:353` drives server level ticks. AGENTS.md:166 explicitly forbids reviving in-frame execution.
* `game.rs:4536,4807,5162`: destruction checks restrictions and held-item capability before `player_destroy_block`; use prioritizes block/item interactions before placement, then applies components, creates block entities and performs post-placement hooks. These private binary methods cannot be imported by horizon-web. Native creative semantics are larger than a storage mutation or even one level call.
* `crates/horizon-world/src/level/place.rs:1674`, `interact.rs:648`, `mod.rs:679`: public `place_block`, `use_without_item`, `player_destroy_block` already implement substantial native behavior. Placement resolves replaceability, facing, survival, obstruction, flags and multi-block placement. Reuse these directly, not default-state writes.
* `level/mod.rs:307,529` and `level/ticks.rs:81`: `LevelAccess` supplies world queries, mutation, RNG, clocks, collision, entities, block entities and effects; `Ticks` owns scheduled work and must register loaded chunks. Defaults such as unobstructed=true and emit=no-op are harness conveniences, not sufficient gameplay implementations.
* `game.rs:1122,1175,1268,3460,3646`: native adapters use `World::set`, contextual collision plus entity obstruction, immediate effect handling and ordered scheduled/random/block/entity work. Scheduled effects are processed between ticks, not merely drained at the end. `game.rs:3974` handles falling blocks and `4039` primed TNT: discarding these destroys gameplay even in creative.
* `crates/horizon-client/src/flatgen.rs:74,276`: public `FlatTerrain::from_settings` and its `ChunkGenerator::{proto,finish}` implementation create native chunks without starting the streaming worker pool. NBT scalar helpers and server JSON parsing currently couple this small module to horizon-client; move them with reexports, retaining one implementation.
* Procedural APIs also exist: `worldgen/generator.rs:351,2808,3004,3099` exposes data loading, generator construction, proto generation and 3x3 decoration. Its loader performs filesystem reads and three real loading threads (`:325,389`); `VanillaTerrain::finish` adds entity/tick/biome conversion (`vanillagen.rs:734`). Calling `generate_proto` alone would miss this integration.

## Minimum ownership and API shape

Create `horizon-creative` below horizon-client, depending on the existing world/render/physics/items/assets crates. Three modules suffice:

* `world`: `CreativeWorld { world: World, player: PlayerMotion, ticks: Ticks, generator: FlatTerrain, profile: BuildProfile }`. One owner mutates world/player state; render code borrows them. Use the existing Timer at 20 Hz and explicit lifecycle pause policy, independent of frames.
* `actions`: extract the admitted native creative action path and contextual obstruction adapter; native server callers use the same functions. Keep native mod hooks, broader interactions and unrelated behavior in their existing locations. No second browser dispatch tree. An opaque `BuildItem` is produced only by profile validation; public actions are move/use/break, not arbitrary block writes.
* `codec`: extract existing chunk and player encode/decode from save.rs, with native reexports. Persist versioned metadata, generator settings, clocks, player and changed/generated chunks. Keep storage out of gameplay.

Browser-only `OfflineWorld { runtime: CreativeWorld, store: BrowserStore, lease: WorldLease }` owns asynchronous persistence around the synchronous domain runtime. `offline_store` owns IndexedDB transactions and exclusive per-world leases, not gameplay. A proposed surface is:

```rust
impl OfflineWorld {
  async fn create(Resources, NewWorld, WorldLease) -> Result<Self>;
  async fn open(Resources, WorldId, BrowserStore) -> Result<Self>;
  fn advance(&mut self, CreativeInput) -> Result<()>;
  fn scene(&self) -> &World;
  fn player(&self) -> &PlayerMotion;
  async fn save(&mut self) -> Result<SavedRevision>;
}
```

Bodies are unimplemented; these signatures specify ownership, not compilable Rust syntax. The small surface hides generation admission, picking, native action ordering, lighting and durability. Avoid a general transport abstraction or packet-shaped local simulation.

## Dataflow and persistence boundaries

Imported resource bytes -> validated common registry/facts -> flat-generated or decoded chunks -> shared motion/actions -> native level mutations -> `World::set` dirty/light/save tracking -> existing lighting/mesher -> renderer. Browser input supplies the same player intent; sockets participate only in remote sessions.

`level/data.rs:690,763,790` still reads behavior JSON, block tags, collision tables and fluid tags from paths. Add a byte/source loader and preserve native wrappers. `horizon-assets/src/source.rs:3` already provides `ResourceSource`; horizon-assets has no world dependency, so it can be reused without a cycle. `light.rs:127` similarly needs source-based loading of occlusion tables; use actual `light_chunk`/`relight`, not BrowserClient's remote light-queue clearing (`client.rs:776`). The existing resource packager omits fluid tags and worldgen bundles (`tools/web_resources.py:22`); extend exactly the resources selected generation/behavior requires.

`save.rs:1168,1421,3298,3423` holds reusable chunk/player codecs; `SaveDir`, `SessionLock`, writer threads and dat-file operations are filesystem shells (`:2074,2329,2610`). `region.rs:27` already exposes byte-backed `ChunkRecord::from_nbt/nbt`; `RegionFile` itself requires a seekable File. Store existing compressed chunk records in IndexedDB; Anvil file import/export is separate work.

Snapshot at a tick boundary, attaching scheduled work with the native `game/persist.rs:88` packing path. Encode first; atomically commit chunk records, metadata and player under one revision. Do not clear dirty generations until that exact revision commits. Retain dirty state on quota/transaction errors; reject incompatible registry/version identities on reopen. Save incrementally, await explicit Save-and-close, and reopen the last committed revision after termination. Unload callbacks cannot promise durability. A second tab must fail exclusive acquisition; transactional owner/revision checks fence stale writers.

## Evidence, tradeoffs and alternatives

Checkout initially clean: `codex/horizon-web`, HEAD `3277bb98164804f8cc00bf7468493df71d6cf455`, tracked fork divergence 0/0. Final HEAD is unchanged, but concurrent edits appeared in `crates/horizon-web/src/input.rs` and `tools/web_touch_check.cjs`. They were left untouched. I made no Git writes or repository edits.

Measured: temporary, source-identical FlatTerrain plus existing JSON/NBT helpers and public level/ChunkRecord calls passed wasm `cargo check --locked --offline` using matching stable rustup Cargo/RUSTC, 29.75 seconds. After concurrent edits appeared, the same probe passed against a separate `git archive` of the exact requested commit (1.45-second cached check). See receipt.json, probe/main.rs, compile.log and pinned-compile.log. It type-checks unused call sites; it neither initializes behavior resources nor runs a world. Filesystem/thread blockers and native semantic gaps above are source findings. Runtime responsiveness, profile closure and persistence are unproved.

We accept a limited palette/superflat generator for a much smaller first runtime. A full Game/dedicated port offers broader parity but exposes startup, socket, thread and platform migration before this milestone. A direct browser Behaviours wrapper is smaller but loses native action policy and side effects; reject it. No synthesis or cross-family review was performed; Claude is unavailable.

## Independently testable units and acceptance

1. Extract source loaders and FlatTerrain unchanged; compare native/source-loaded tables and generated chunks, then wasm-check. Reject missing or mismatched data explicitly.
2. Extract shared native creative actions and obstruction logic. Differentially test admitted placements, collision rejection, slab merging, facing, fence connections, break results and event order against the current server path. Prove palette/event closure; unexpected stateful effects fail visibly rather than disappear.
3. Extract codecs; round-trip chunks, scheduled work and player position/rotation/velocity/flight/selection. Preserve native save fixtures. Test rollback, quota errors, stale writer and interrupted commit.
4. Add OfflineWorld and browser storage; run real-browser acceptance with **all WebSockets blocked**: import owned resources, create a generated world, walk/fly, place/break across chunk boundaries, await Save-and-close, reopen after reload, verify edits and player state, then exercise failed saving and recovery. Assert zero socket attempts and inspect actual Horizon rendering.
5. Rerun remote two-client convergence/persistence and relevant native server-thread checks after shared extraction.

HTTP delivery of HTML/Wasm/resources remains allowed. No service worker or full-app offline cache is included. This completes the design inspection; implementation and offline browser acceptance remain outstanding.
