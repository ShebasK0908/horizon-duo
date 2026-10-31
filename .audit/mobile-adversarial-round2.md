# Fresh runtime review of mobile input fix round 1

**FAIL: one additional P2. Both original P2 corrections PASS.**

Strict range: `2623ca8d875d7a5b81d0c17b61bae04699d3cd01..728012fa214cf2cd7a27b1a07f96af380d82d75d`.
Source worktree: /Users/test/.t3/scratch/2026-10-25-compile-this-for-the-iphone-cf98d4f8/src/horizon-web. Initial branch `codex/horizon-web`, clean and synchronized 0/0, HEAD `728012fa214cf2cd7a27b1a07f96af380d82d75d`. Read AGENTS.md, CONTRIBUTING.md, both requested audit reports, parsed complete mobile-input-repros.json, docs/web-port-status.md, implementation diff, committed touch driver, and original reviewer scripts. Fixed written review scope; no subagents.

## Route and artifact

Real desktop Chrome WebGPU with Playwright CDP emulated touch and actual keyboard events. Owned resource ZIP imported. Fresh disposable native server on 25589 and owned bridge on 18789. Gameplay URL: `http://127.0.0.1:8766/?world=webproof&bridge=ws://127.0.0.1:18789`. Gameplay used this local route; this is not an additional Tailscale gameplay run.

Private Tailscale HTTPS HTML, JS, and wasm all matched local bytes at the start and finish. wasm SHA-256 `aaab226ef53e6e9abf33898e3223ab822fe9dbf42211f1aba914ce2ea63e14d0`. The original repro also compared the actual local HTTP responses to these files. This is served artifact identity, not fresh source-build provenance. No rebuild was performed.

RAF instrumentation gates callbacks and logs events; it does not alter Rust memory, physics, or network state. Observer positions are independently received from the native server.

## Original failures: PASS

- Stalled separated Jump taps: pointerdowns 8929.0 and 9650.100000023842 ms, **721.1 ms apart**. Flight remains false. Client stays `[2.5,-60,-1.5]`; observer entity 27 receives the same position. Normal-render control and subsequent single hold also leave flight false. `focused-v3.json` records these cases. Committed driver repeats this at a 717.5 ms gap and passes.
- Fresh touch after trusted tab blur/focus: blur 7657.0 ms, focus 7764.5 ms, new Forward pointerdown 7776.800000071526 ms. After 700 ms, client moves `[-5.5,-60,-3.5]` to `[-5.5,-60,-0.09]`, **3.41 blocks**. Observer entity 171 receives `[-5.5,-60,-0.305908203125]`, with ordinary delivery lag. Document is focused and visible. Repress control also moves.
- Fresh W after trusted tab blur/focus: blur 10385.700000047684 ms, focus 10493.800000071526 ms, then fresh KeyW. Client moves `[-5.5,-60,3.6158]` to `[-5.5,-60,7.0259]`, **3.4101 blocks**; observer receives `[-5.5,-60,6.81005859375]`. `trusted-focus-v2.json` records events and positions. Committed driver independently asserts both fresh input paths and two trusted Chrome blur/focus pairs.

## Additional runtime-proven defect

**P2: cancelling a Jump touch erases an earlier completed keyboard jump.**

Location: `crates/horizon-web/src/input.rs:91-93`, especially `self.jumps.clear()` at line 92; keyboard now shares this queue through `BrowserInput::key`, and `client.rs` exclusively consumes its sampled Jump.

Trigger: stall RAF before the next tick, press/release Space, press Jump with a touch, issue touchCancel, resume RAF. A completed keyboard tap should survive cancellation of the independent pointer.

First confirmation has trusted Space keydown at 7110.399999976158 ms and keyup at 7111.100000023842 ms; trusted Jump pointerdown at 7132.399999976158 ms; trusted pointercancel at 7147.899999976158 ms. Client starts `[-3.5,-60,1.5]` and stays at height -60 for all 22 samples over about 900 ms. Observer also stays at `[-3.5,-60,1.5]`. Independent repeat at `[6.5,-60,-4.5]` also stays grounded.

Keyboard-only control rises from -60 to **-58.7478**; observer receives height **-58.750732421875**. Replacing the cancelled Jump touch with a cancelled Forward touch preserves that jump, with the same client peak and observed server ascent. The initial adversarial matrix independently reproduces the loss.

Impact: a legitimate queued keyboard jump is silently discarded when a separate Jump touch is cancelled during a delayed frame.

Diff attribution: Space and key_presses previously fed Jump independently in client.rs. The fix moves keyboard transitions into the aggregate queue; the blanket pointer-cancellation clear now discards keyboard events as well.

Smallest correction: scope cancellation to the cancelled pointer's pending gesture, preserving unrelated completed keyboard transitions and existing cancellation semantics for the pointer.

Receipts: `cancellation-confirm.cjs/json/log`, cases `keyboard-quick-canceled-jump` and `keyboard-quick-canceled-jump-repeat`; controls `keyboard-quick-control` and `keyboard-quick-canceled-forward-control`.

## Completed probe ledger

| Item | Result | Evidence |
|---|---|---|
| Original stalled Jump failure | PASS | focused-v3.json; baseline/browser.json |
| Fresh trusted-focus touch and keyboard | PASS | trusted-focus-v2.json; baseline/browser.json |
| Queued single touch and Space taps | PASS | adversarial.json, server-observed ascent |
| Valid touch/keyboard double taps | PASS | adversarial.json, flight true and observer elevation |
| Both key-to-touch and touch-to-key double taps | PASS | adversarial.json |
| Long stalled touch, keyboard and mixed gaps | PASS: no flight | focused-v3.json; cancellation-confirm.json |
| Held/mixed Jump ownership | PASS | flight-owners.json, all four cases |
| Cancelled touch-only Jump | PASS | adversarial.json |
| Cancellation after completed queued keyboard tap | FAIL, P2 above | cancellation-confirm.json, repeated |
| Gameplay and four viewport sizes | PASS | baseline/browser.json passed=true |
| Desktop two-client movement, edits and reconnect | PASS | desktop/browser.json passed=true |
| Fixed tick cadence | PASS | ticks/browser.json passed=true |
| Artifact identity and cleanup | PASS | pin-start.json, pin-final.json |
| Physical phone, Safari, Duo | COULD NOT RUN | DEVICE_IN_USE respected; no device operation |
| Offline, X execution, native platform client/full parity | COULD NOT RUN | outside this browser input review |
| Cross-family review | COULD NOT RUN | configured Codex reviewer; Claude unavailable, not retried |

The flight ownership test releases/cancels one source and verifies the remaining source continues ascent, then releases the final source and verifies stopping. Keyboard release preserves touch, touch release preserves keyboard, touch cancellation preserves a still-held keyboard, and first touch release preserves the second touch. Each remaining owner produces about 3.748 blocks of ascent over 500 ms; final vertical drift is 0. Observer positions are included.

The committed touch baseline covers movement/release, simultaneous look, actual CDP touchCancel, lost capture, blur cleanup, synthetic visibility cleanup, keyboard/touch movement ownership, two Forward owners, quick retained jump with 250 ms delayed callbacks, native flight ascent/descent, authoritative gold placement/removal, and viewports 390x844, 844x390, 1152x804, 320x568. No GPU/page errors. Desktop baseline covers two clients' movement, gold placement, reconnect and removal. Tick cadence: 50 ticks/2503 ms normally and 51 ticks/2502 ms with 250 ms callback delay; these are cadence checks, not performance benchmarks.

Harness limitation: two first-matrix long-gap cases also required a sampled jump apex after resuming. Catch-up can advance much of a jump before the first sample, so these raw cases have pass=false for that unnecessary observation. Neither shows flight. Both were rerun with the intended no-flight assertion in cancellation-confirm.cjs and pass. Raw scripts and outputs remain preserved. This is not an additional product defect.

## Exact execution commands

From /Users/test/.t3/scratch/2026-10-25-compile-this-for-the-iphone-cf98d4f8/src/horizon-web, with `D=/var/folders/t6/mlxmtrgs5fb7th7lvjzsln8h0000gn/T/horizon-fix-review-tmde31in`:

```sh
python3 "$D/run_server.py" --resources .local/web-owned/26.3 --port 25589
target/debug/horizon-web-bridge --bind 127.0.0.1:18789 --world webproof --upstream 127.0.0.1:25589 --origin http://127.0.0.1:8766
node "$D/focused-v3.cjs" > "$D/focused-v3.log" 2>&1
node "$D/trusted-focus-v2.cjs" > "$D/trusted-focus-v2.log" 2>&1
node "$D/adversarial.cjs" > "$D/adversarial.log" 2>&1
node "$D/cancellation-confirm.cjs" > "$D/cancellation-confirm.log" 2>&1
node "$D/flight-owners.cjs" > "$D/flight-owners.log" 2>&1
```

For committed drivers, each invocation used the following common environment directly:

```sh
export HORIZON_PLAYWRIGHT=/Users/test/.npm/_npx/705bc6b22212b352/node_modules/playwright
export HORIZON_WEB_BUNDLE=.local/web-owned/play-resources.zip
export HORIZON_WEB_URL='http://127.0.0.1:8766/?world=webproof&bridge=ws://127.0.0.1:18789'
HORIZON_WEB_PROOF="$D/baseline" node tools/web_touch_check.cjs > "$D/baseline.log" 2>&1
HORIZON_WEB_PROOF="$D/ticks" node tools/web_tick_check.cjs > "$D/ticks.log" 2>&1
HORIZON_WEB_NAME_PREFIX=R1Rv HORIZON_WEB_PROOF="$D/desktop" node tools/web_play_check.cjs > "$D/desktop.log" 2>&1
```

Original reviewer scripts were copied unchanged except for their disposable local bridge URL. run_server.py is a scratch copy of tools/run_web_test_server.py with /Users/test/.t3/scratch/2026-10-25-compile-this-for-the-iphone-cf98d4f8/src/horizon-web set to the repository and its output root redirected from repository .local into this evidence directory. Its isolated HORIZON_HOME, launcher home, XDG data/config, no-update and no-audio behavior remains intact. No Rust command was needed; no build or source modification occurred.

## Final integrity and cleanup

All owned Chrome sessions closed in finally. Temporary T3 preview tab closed. Owned native server PID 45302 stopped normally, exit 0; owned bridge PID 45318 terminated. Both PIDs absent and ports 25589/18789 closed. Protected PIDs 55502, 88366, 88504, 9771 retain their original commands. Tailscale configuration untouched. No PR, push, merge, deploy, external message, device action, or repository edit by this reviewer.

An external actor advanced the shared branch during the run to `3277bb98164804f8cc00bf7468493df71d6cf455` (Player Card tooling/assets/docs). Final worktree is clean and upstream divergence 0/0. `git diff 728012fa2..HEAD -- crates/horizon-web tools/web_touch_check.cjs` is empty; final served HTML/JS/wasm hashes are unchanged. The new card files and documentation are outside this review. Verdict applies strictly to the requested range ending at 728012fa2. pin-final.json records the external advance and full changed-file summary.

Additional unrun surfaces: actual OS background suspension, phone GPU/folding/safe-area behavior, public hosting. Native dedicated server execution is a test fixture, not native platform-client or full native-parity evidence.
