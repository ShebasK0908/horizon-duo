Read-only review of a12b5151e139b346aef09c3ceacb4e5f925694a8..e22aea28a3c8baf017fd802ffd16c0a0082c0ee7

Request changes. Four verified browser-state/timing findings follow. No source file was changed by this reviewer. All repro source, world data, profiles and receipts were placed in this mktemp directory. No subagents or GitHub writes were used.

Repro environment: native server 25586, bridge 18786, existing read-only HTTP 8766, owned resources /Users/test/.t3/scratch/2026-10-25-compile-this-for-the-iphone-cf98d4f8/src/horizon-web/.local/web-owned/26.3 and play-resources.zip. Existing 25575/8767/18767/8766 services were not restarted or stopped. The scratch scripts start and cleanly stop their own native server. During the runs, the scratch bridge was started with this exact command from the repository:

```sh
target/debug/horizon-web-bridge --bind 127.0.0.1:18786 --world reviewproof --upstream 127.0.0.1:25586 --origin http://127.0.0.1:8766
```

The checkout advanced concurrently, first to db474573e73c57a3099559dd880dc16ddcb533fb and finally observed at 98fb29681bfeed54b3b8a624f7c22922bcd98ebf. The findings remain anchored to e22aea28; `git diff e22aea28 -- crates/horizon-web/src/client.rs crates/horizon-client-physics crates/horizon-client/src/save.rs` was empty. Browser runs used the served WebAssembly, rebuilt concurrently with renderer-only changes; its observed SHA-256 was 0ce1f5a8c1018485e2c6e2caad6a676e85f1bd52db09786ba2ae3eab615debd6. This is runtime evidence for the unchanged affected browser code, not an independently rebuilt exact-e22 artifact.

1. [P2] Apply the server's own-player UpdateAttributes packets.

Location: crates/horizon-web/src/client.rs:582 (unhandled packet fallback), with movement driven by MotionWorld at lines 680-690. The new packet handler drops UpdateAttributes rather than applying its instances to PlayerMotion.attributes. The native client already applies these packets at game/remote.rs:3026.

Exact executed commands:

```sh
REVIEW_SPEED=0.5 node /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/attribute-repro.cjs > /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/attribute-ground-repro.out 2>&1
/var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/read-packets /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/attribute-ground-packets.bin
REVIEW_SPEED=0.1 node /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/attribute-repro.cjs > /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/attribute-control-repro.out 2>&1
/var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/read-packets /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/attribute-packets.bin
```

The scratch fixture patch changes only the offline test player's saved movement_speed and clears its saved flying flag. The actual server loads that native NBT and sends the update. Incoming WebSocket bytes are captured and decoded by the real horizon-protocol ClientSession in read-packets.rs. Decoded output:

```text
SELF_ENTITY 85
SELF_ATTRIBUTES ... AttributeSnapshot { attribute: 26, base: 0.5, modifiers: [] } ...
SELF_ATTRIBUTES ... AttributeSnapshot { attribute: 26, base: 0.1, modifiers: [] } ...
```

Registry index 26 in the owned attributes.json is minecraft:movement_speed. For each 3000 ms W hold, actual browser positions were:

```text
base 0.5: 40.5000,-60.0000,0.5000 -> 40.5000,-60.0000,10.6416
base 0.1: 40.5000,-60.0000,0.5000 -> 40.5000,-60.0000,10.8575
```

Expected: the shared simulation uses the authoritative base speed, with approximately five times the displacement at 0.5 for comparable ticks. Actual: the speed remains effectively unchanged, and the other browser receives the slow positions. Impact: native durable player attributes survive server loading and reach the browser but do not control its gameplay. Apply the received attributes to the current player, retaining initial updates if they precede player creation. Packet receipts: attribute-ground-packets.out, attribute-control-packets.out; raw behavioral observations: attribute-ground-repro.out and attribute-control-repro.out.

2. [P2] Update active game mode when GameEvent(3) arrives.

Location: crates/horizon-web/src/client.rs:582 and :587. game_type is assigned only by dimension(), and player.spectator is derived from that stale value at :511. GameEvent is discarded.

Exact executed command:

```sh
node /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/mode-repro.cjs > /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/mode-repro-confirm.out 2>&1
```

The script joins fresh creative profiles, builds a wall at z=3, then executes `gamemode spectator RevModeB` and repeats a 2000 ms W hold. Actual output:

```text
CREATIVE_WALL position=8.5000,-60.0000,2.7000
SPECTATOR_WALL position=8.5000,-60.0000,2.7000
server: Set RevModeB's game mode to Spectator Mode
server: RevModeB has the following entity data: 3
```

Expected: spectator movement crosses the wall. Actual: the browser remains collision-bound at its face. Impact: changing server game mode leaves browser movement and interaction rules in the previous mode. Handle GameEvent event 3 and update the existing player's mode-dependent state immediately. Raw receipts: mode-repro.out and mode-repro-confirm.out; console receipt: mode-server.log. A confirmation run with the same mode-repro.cjs script preserves the mode change and data-query output in mode-repro-confirm.out.

3. [P2] Invalidate the selected creative stack after authoritative inventory changes.

Location: crates/horizon-web/src/client.rs:826. selected_item caches only the material name; incoming inventory packets never invalidate it. A subsequent placement skips SetCreativeModeSlot even when the server removed the held stack.

Exact executed command:

```sh
node /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/browser-repro.cjs > /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/browser-repro.out 2>&1
```

Steps: place gold; break it; server `clear RevOctA`; press F with gold still selected; then change selection to diamond and press F. Actual observations:

```text
BASELINE_PLACE target=[[0,-60,2],[0,0,-1],3830]
server: Removed 64 item(s) from player RevOctA
AFTER_CLEAR_SAME_SELECTION target=[[0,-61,2],[0,1,0],9]
last authoritative update at [0,-60,2] is state 0
AFTER_CHANGE_SELECTION target=[[0,-60,2],[0,0,-1],6865]
```

Expected: selecting gold in creative supplies gold when placing, including after a server inventory edit. Actual: repeated placements with the same visible selection fail until the user changes material or reconnects. Impact: the material selector lies about what the server will place after clear/replacement. Invalidate or reconcile the cache for authoritative held-slot/inventory updates, or send the selected creative stack on each placement. Receipts: browser-repro.out and first-server.log.

4. [P2] Preserve elapsed simulation ticks across redraws.

Location: crates/horizon-web/src/client.rs:931-939. Each redraw runs at most one 50 ms simulation tick, then resets last_tick to now, discarding elapsed time. Slow rendering changes movement speed and interpolation rather than only frame smoothness.

Exact executed command:

```sh
node /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/clock-and-check-repro.cjs > /var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-web-review-7aawg_fm/clock-and-check-repro.out 2>&1
```

This uses fresh creative profiles on level ground. It holds W for 3000 ms, waits 500 ms for settling, then repeats after adding 250 ms delay to requestAnimationFrame callbacks. Outgoing actual ClientTickEnd packets are counted through Playwright WebSocket frame events. Actual output:

```text
FRAME_NORMAL elapsed=3515 ticks=54 position=56.5000,-60.0000,10.6416
FRAME_CAP_250MS elapsed=3515 ticks=27 position=56.5000,-60.0000,5.4417
both start at 56.5000,-60.0000,0.5000
```

Expected: comparable simulation time and movement for equal input durations; lower FPS affects rendering. Actual: half the simulation ticks and about half the movement, with the observer receiving the shorter travel. Impact: creative movement and remote interpolation depend on rendering throughput. Accumulate elapsed time and advance fixed ticks without discarding the remainder. The exact committed e22aea28 tools/web_play_check.cjs was copied unchanged to committed-play-check.cjs and ran successfully against this scratch server: COMMITTED_PLAY_CHECK 0, with committed-play-proof/browser.json reporting passed=true. The committed runtime check at tools/web_play_check.cjs:30 only waits for more than 3 blocks within 15 seconds; it never verifies the simulation rate. Its peer-position assertion reads the network target, not the rendered interpolated position. These are blind spots tied to the reproduced timing defect, not separate hypothetical findings.

Validation and scope receipt:

- Inspected the full 103-file committed diff and docs/web-port-status.md, plus the committed audit receipts for rendering/import, movement, two clients, block updates, wrong-resource rejection, persistence and tailnet routing.
- Native save/physics extraction: compared moved implementations against the base. No runtime-verified native regression found. Resource-backed saved-attribute round trip passed (1); shared physics passed (39); protocol passed (126); renderer passed (627, 16 existing ignored).
- Native binary check and wasm32 horizon-web check passed. Rust commands used RUSTC=/Users/test/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc and /Users/test/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo, with --locked. Detailed receipts: shared-tests.out, render-tests.out, native-check.out, wasm-check.out. Saved-motion test command: cargo test --locked -p horizon-client --lib motion_saved_tests -- --nocapture with HORIZON_TEST_RESOURCES set to the owned 26.3 fixture; output was 1 passed, 0 failed.
- Browser probes used actual Chrome WebGPU and the actual native Horizon server/bridge. Packet traces used actual horizon-protocol decoding. No visual claim is inferred merely from mesh/vertex counters.
- The exact committed two-client browser check passed independently: committed-play-proof/browser.json, with placement, reconnect and breaking receipts. This successful baseline does not cover the additional reproduced state changes or simulation-rate checks.
- Existing audit persistence/tailnet receipts were reviewed; no independent tailnet or public-X deployment test was performed. X embedding and phone controls are acknowledged incomplete and are not findings here. Offline browser persistence remains unfinished as documented.

Final cleanup: the owned server 25586 exited cleanly and the owned bridge 18786 was stopped. lsof found neither owned listening port. The pre-existing native server 25575 (PID 88504), original bridge 8767 (PID 88366), parent bridge 18767 (PID 9771), and HTTP 8766 (PID 55502) were still listening with their original PIDs. The reviewer made no source, branch, commit or push changes.
