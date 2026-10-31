# Fresh Horizon artifact verification

The goal was real Horizon browser gameplay with authoritative multiplayer through the private Tailscale runtime. This fresh verifier ran the served artifact and all four prior browser-state reproductions. No source fixes, commits, pushes, upstream posts, PRs, service reconfiguration, or Tailscale changes were made.

Pinned source was clean pushed `6ee463b8d8ec30470771c6dcec688a36c239dc33` on `codex/horizon-web`, with zero divergence from `fork/codex/horizon-web`. Source review was limited to the requested `98fb29681..HEAD` fix diff, together with the specifically requested reports. Existing parent test receipts were read as parent evidence, not counted as independent test execution.

## Results

| Item | Verdict | Actual observation |
| --- | --- | --- |
| Main artifact | PASS | Actual Chrome WebGPU through Tailscale HTTPS/WSS. Both clients received gold state 3830 and air state 0. Reconnect loaded 3830. Rendered placement and removal were inspected. Separately aimed captures show textured remote players in both directions. |
| R1 | PASS | Saved-NBT speed 0.5 travelled 65.8314 blocks; 0.1 travelled 12.9478, ratio 5.0844. Both initial UpdateAttributes deliveries decoded. |
| R2 | PASS | Creative stopped at z=2.7000; spectator crossed the same z=3 wall to z=20.1834. Server confirmed game type 3. |
| R3 | PASS | After server `clear RevOctA`, unchanged gold selection produced authoritative state 3830 at [0,-59,1]. |
| R4 | PASS | Normal 3514 ms, 70 ticks, 12.9478 blocks. Delayed 3502 ms, 72 ticks, 12.0844 blocks. Both observers received the final positions. |

The separate committed `tools/web_tick_check.cjs` passed through Tailscale. It counted 50 outgoing ClientTickEnd packets in 2502 ms with normal callbacks, and 51 in 2501 ms with 250 ms callback delays. These are fixed-tick correctness observations, not a performance benchmark.

## Artifact identity

Served HTML, JavaScript, and wasm were HTTP 200 and byte-identical to their local `web/` files, independently checked through both Tailscale and localhost 8766. The wasm was 21,331,663 bytes and SHA-256 `9e3d397978bf7022f62bb25b59c2c72aa1293f490d925cd6e1bbf30ee4776575`, matching the prior manifest. See `served-artifacts.json` and `local-served-artifacts.json`.

## Commands and evidence

`commands.sh` contains the exact main and reproduction invocations, including the separately tokenized owned bridge arguments. Every script finished with exit 0. `assert-results.py` then asserted actual outcomes from the receipts, rather than trusting these exits. Its output is `assertions.out`, and its structured result is `results.json`.

The reproduction runners are scratch copies of the supplied scripts. Only the Playwright require location, fixture directory binding, and report destinations were adapted. All native source root pointers and behavior are unchanged. `runner-adaptations.json` records the original script hashes and verifies those adaptations. Original fixture scripts were not edited. The original fixture server.log was backed up before any fixture run, and restored after preserving per-run logs. Existing .out reports, raw packet files, browser-logs.json, and mode-server.log were preserved by routing scratch-runner writes to the proof directories. The synthetic fixture world was intentionally exercised and saved by the native servers.

- `tail-play/browser.json` is the exact committed two-client script result. Names used the unique prefix `FreshLoG`.
- `tail-play/placed.png` and `tail-play/broken.png` were visually inspected. The gold cube appears, then is absent after the authoritative removal.
- `tail-play/peer.png` does not show a peer because that player was behind the observer camera. It is not used as visible-peer proof.
- `peer-visual/alpha-facing-beta.png` and `peer-visual/beta-facing-alpha.png` were both visually inspected. Both show the textured remote player. `peer-visual.cjs` used fresh `VisLoGAlpha` and `VisLoGBeta` names and actual keyboard camera controls to face each other. `peer-visual/browser.json` records those positions and camera targets.
- `tail-tick/browser.json` and `tail-tick.out` preserve the committed Tailscale tick check.
- `mode/run.out` and `mode/mode-server.log` preserve the creative/spectator wall reproduction.
- `attribute-0.5/run.out` and `attribute-0.1/run.out` preserve the saved-NBT treatment and control. Each directory contains the captured raw `attribute-packets.bin`, decoded `packets.out`, and native `server.log`.
- `inventory/run.out` and `inventory/server.log` preserve the clear-and-replace result.
- `clock/run.out` and `clock/server.log` preserve elapsed windows, actual outgoing packet counts, travel, and observer positions.

The decoded saved-NBT treatment included `SELF_ENTITY 101` and `AttributeSnapshot { attribute: 26, base: 0.5, modifiers: [] }`. The control included `SELF_ENTITY 110` and `AttributeSnapshot { attribute: 26, base: 0.1, modifiers: [] }`. The inventory script's extra CLI attribute command produced unchanged travel. That command is not used as evidence of attribute delivery or as the R1 treatment. R1 relies exclusively on the saved-NBT probes and captured incoming protocol frames.

## Limits

This is independent runtime artifact verification of the four reproduced P2 failures, not a full native or browser parity audit. The native physics 40, clock 8, native binary, wasm build, and Clippy checks were not rerun here. Their existing parent receipts remain parent-supplied evidence. The served build was compared with the local artifact, not independently rebuilt from source.

Initial native attribute delivery and its movement effect were exercised. A synthetic forced ordering of attributes before browser player construction was not separately injected. Spectator wall crossing and vertical movement were exercised; spectator F/R action suppression was source-reviewed but not separately asserted by these supplied runners.

Phone controls, mobile WebGPU, offline browser persistence, and X embedding remain acknowledged open work. They are not new findings from this run.

The Prove It Works principle changed the verification choice. This run checked real packet delivery, actual state changes, visible rendered peers, and placement/removal captures instead of accepting counters or earlier pass claims.

## Cleanup

Original port 25575 still listens under unchanged PID 88504.
Original port 8767 still listens under unchanged PID 88366.
Original port 18767 still listens under unchanged PID 9771.
Original port 8766 still listens under unchanged PID 55502.
Owned port 25586 has no listener.
Owned port 18786 has no listener.
Source HEAD remains 6ee463b8d8ec30470771c6dcec688a36c239dc33, clean, with upstream divergence 0/0.
Original fixture server.log restored byte-for-byte; each run log preserved separately.
Only the owned 18786 bridge was stopped. The preview tab opened by this verifier and all Playwright browsers were closed.
