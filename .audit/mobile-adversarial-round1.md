Read-only review of Horizon mobile/browser lifecycle diff

Reviewed range: `aba56c78dd2c2974c64e47c2b448ee8925b1dab3..1fc53d53515e41a1d9d90cbbe1dff7ed85d3490b`.
Source: `/Users/test/.t3/scratch/2026-10-25-compile-this-for-the-iphone-cf98d4f8/src/horizon-web`. Branch: `codex/horizon-web`.
Initial `git status --short --branch`: `## codex/horizon-web...fork/codex/horizon-web`, with no changed files.
Upstream divergence: `0 0`. Read `AGENTS.md`, `CONTRIBUTING.md`, `docs/web-port-status.md`, `.audit/mobile-verification.md` and the complete implementation diff.
Mode: fixed, written review scope. No repository edits, builds, commits, pushes, deployments, device operations, external posts, subagents or cross-family substitutions.

Two P2 findings were demonstrated against the served browser artifact, using actual CDP touch/Playwright keyboard input and server-delivered observer state. JavaScript instrumentation gated rendering callbacks and recorded DOM events; it did not modify Rust memory or gameplay state.

1. **P2: Preserve elapsed time between retained Jump taps.**

Location: `/Users/test/.t3/scratch/2026-10-25-compile-this-for-the-iphone-cf98d4f8/src/horizon-web/crates/horizon-web/src/input.rs:91` (also lines 38, 50, 83, 92-94), consumed by `client.rs:998` in the simulation catch-up loop.
`TouchInput` stores only booleans and pops one on each simulation tick. When redraw callbacks stall, two well-separated completed taps become `true,false,true,false` in consecutive catch-up ticks. The existing native seven-tick double-tap detector then enters creative flight.

Exact reproduction command, run from the source worktree:

```sh
node /tmp/horizon-mobile-review.FT1JRF/focused-v3.cjs > /tmp/horizon-mobile-review.FT1JRF/focused-v3.log 2>&1
```

The script joins with fresh profiles, imports the supplied owned ZIP, first runs two ordinary taps separated by 700 ms, then gates requestAnimationFrame callbacks, delivers the same two real touches with a 700 ms wait, resumes callbacks, and reads both client state and another client's server-delivered peer positions.

Expected: these are separate jumps, more than 350 ms apart, so flight remains false just as it does in the normal-render control.
Actual control: `separated-normal.after.flying="false"`, position `[-6.5,-60,-9.5]`.
Actual stalled input: pointerdown timestamps `9942.300` and `10663.100` ms, a `720.800` ms gap. `separated-stalled.after.flying="true"`, position `[-6.5,-58.2575,-9.5]`; observer receives entity 2245 at `[-6.5,-58.257568359375,-9.5]`.
A subsequent single Jump hold rises to `[-6.5,-53.7631,-9.5]`, with observer position `[-6.5,-53.766845703125,-9.5]`. This confirms a lasting flight-mode change, not only a stale diagnostic flag.

Smallest correction: retain event timing when replaying jump transitions, or expire/reset stale gesture history before replay, while preserving a short tap until a real simulation tick consumes it. Do not replay temporally separate presses as adjacent double-tap candidates.
Receipts: `focused-v3.json` / `focused-v3.log`, tests `separated-normal`, `separated-stalled`, `held-jump-after-stall`.

2. **P2: Do not let a deferred lifecycle reset erase fresh input after focus returns.**

Location: `/Users/test/.t3/scratch/2026-10-25-compile-this-for-the-iphone-cf98d4f8/src/horizon-web/crates/horizon-web/src/client.rs:925` (lines 925-926), paired with `browser_input.rs:83` (lines 83-85) and `client.rs:325` (`reset_input`).
The DOM blur handler immediately clears touch state and sets a reset flag. On the next draw, `reset_input()` clears touch and keyboard again, including any new pointerdown or keydown received after focus returned. A touch owner is then gone until the user lifts and presses again.

Exact reproduction command, run from the source worktree:

```sh
node /tmp/horizon-mobile-review.FT1JRF/trusted-focus-v2.cjs > /tmp/horizon-mobile-review.FT1JRF/trusted-focus-v2.log 2>&1
```

The script disables Playwright's focus emulation with CDP `Emulation.setFocusEmulationEnabled`, gates RAF callbacks, switches to an owned blank Chrome tab and back with `bringToFront`, presses Forward after returning, then resumes rendering. It repeats with W. Events from the real tab switch have `isTrusted=true`; no synthetic blur is needed for this proof.

Expected: cleanup removes input held before losing focus; a new input received after returning is accepted.
Actual touch: trusted blur at 8798.000 ms, trusted focus at 8904.500 ms, fresh Forward pointerdown at 8924.200 ms. After holding 700 ms, position stays `[-5.5,-60,-7.5]`; observer entity 2422 also remains `[-5.5,-60,-7.5]`. Document has focus and is visible.
Control: lifting and repressing Forward moves to `[-5.5,-60,-4.2659]`, observer `[-5.5,-60,-4.265869140625]`, a 3.2341-block move.
Actual keyboard: trusted blur at 11604.200 ms, trusted focus at 11713.000 ms, fresh KeyW at 11722.900 ms. After holding 700 ms, position and observer remain `[-5.5,-60,-4.2659]` / `[-5.5,-60,-4.265869140625]`.

Smallest correction: reset each source once at the lifecycle boundary, or scope deferred resets to the input generation held at that boundary. A later draw must preserve post-focus input. Avoid fixing only touch, since fresh keyboard input is also erased.
Receipts: `trusted-focus-v2.json` / `trusted-focus-v2.log`. `followup-v3.json` independently reproduces the same touch failure at the DOM blur boundary.

Reviewed scope ledger:

| Scoped item | Review state | Measured evidence |
| --- | --- | --- |
| Touch ownership | Browser exercised | Baseline two owners; independent first-look ownership and rearm after cancel passed. |
| Retained tap transitions | Browser exercised; P2 found | Normal/queued quick tap pass; separated stalled taps spuriously toggle flight. |
| Simultaneous movement/look | Browser exercised | Baseline passes. |
| Cancellation/lost capture | Browser exercised | Actual CDP cancellation and DOM releasePointerCapture; jump overlap/cancel and look rearm pass. |
| Keyboard merging | Browser exercised; lifecycle P2 found | Movement and flight preserve either still-held source; fresh post-focus W is erased. |
| Startup, resize, errors | Browser exercised | Fixture, four gameplay sizes, startup resize, no GPU, failed wasm load, wrong-resource rejection pass. |
| Desktop regression | Browser exercised; lifecycle P2 found | Two-client movement, placement, breaking, reconnect pass; tick cadence passes; post-focus input race affects keyboard too. |
| Phone / Safari / Duo | Blocked | DEVICE_IN_USE respected; no physical-device execution. |
| Cross-family review | Unrun | This is the configured Codex reviewer only; unavailable Claude was not retried. |

Supporting measured checks:

- `baseline/browser.json`: passed. CDP touch quick jump at 250 ms callback delay rises from -60 to -58.7478. Flight rise/descent and server gold placement state 3830 then air 0 observed. Viewports 390x844, 844x390, 1152x804, 320x568 pass size/bounds checks.
- `desktop/browser.json`: passed. Observer sees movement; both clients see block edits. Reconnect sees gold state 3830 at `[0,-60,2]`, then both clients observe its removal.
- `ticks/browser.json`: passed. 50 ticks / 2500 ms with normal callbacks, 51 ticks / 2502 ms with 250 ms delayed callbacks.
- `fixture/browser.json`: passed, three frames and 18 quads.
- `startup.json`: no GPU gives enable/update/reload action; missing wasm gives a visible fetch failure; wrong ZIP is rejected with `Resource manifest does not match Horizon's prepared resource format`. Resize during asynchronous graphics initialization gives canvas 390x591, CSS 390x590.796875, 18 quads, no error.
- `followup-v3.json`: second look pointer is ignored while the first owns look; first pointer changes view; look rearms after cancel; overlapping Jump owners do not toggle flight; cancellation releases; keyboard/touch flight source merging works.

Served identity: the private HTTPS response bytes for HTML, JS and wasm match local `web/` files. SHA-256 values are in `focused-v3.json.artifacts`; wasm is `2afe5825e9baca8c3ba29b746c77582fd553423e8547859f1c9c2a5773955d3c`. This review did not rebuild the source, so this is served/local artifact identity, not a fresh build-provenance claim.

Browser commands used for supporting checks (from the worktree):

```sh
HORIZON_PLAYWRIGHT=/Users/test/.npm/_npx/705bc6b22212b352/node_modules/playwright HORIZON_WEB_BUNDLE=.local/web-owned/play-resources.zip HORIZON_WEB_URL='https://tests-mac-studio.tail935cc1.ts.net:8491/?world=webproof&bridge=wss%3A%2F%2Ftests-mac-studio.tail935cc1.ts.net%3A8492' HORIZON_WEB_PROOF=/tmp/horizon-mobile-review.FT1JRF/baseline node tools/web_touch_check.cjs > /tmp/horizon-mobile-review.FT1JRF/baseline.log 2>&1
HORIZON_PLAYWRIGHT=/Users/test/.npm/_npx/705bc6b22212b352/node_modules/playwright HORIZON_WEB_BUNDLE=.local/web-owned/play-resources.zip HORIZON_WEB_URL='https://tests-mac-studio.tail935cc1.ts.net:8491/?world=webproof&bridge=wss%3A%2F%2Ftests-mac-studio.tail935cc1.ts.net%3A8492' HORIZON_WEB_NAME_PREFIX=RvX32 HORIZON_WEB_PROOF=/tmp/horizon-mobile-review.FT1JRF/desktop node tools/web_play_check.cjs > /tmp/horizon-mobile-review.FT1JRF/desktop.log 2>&1
HORIZON_PLAYWRIGHT=/Users/test/.npm/_npx/705bc6b22212b352/node_modules/playwright HORIZON_WEB_URL='https://tests-mac-studio.tail935cc1.ts.net:8491/' HORIZON_WEB_BUNDLE= HORIZON_WEB_PROOF=/tmp/horizon-mobile-review.FT1JRF/fixture node tools/web_render_check.cjs > /tmp/horizon-mobile-review.FT1JRF/fixture.log 2>&1
HORIZON_PLAYWRIGHT=/Users/test/.npm/_npx/705bc6b22212b352/node_modules/playwright HORIZON_WEB_BUNDLE=.local/web-owned/play-resources.zip HORIZON_WEB_URL='https://tests-mac-studio.tail935cc1.ts.net:8491/?world=webproof&bridge=wss%3A%2F%2Ftests-mac-studio.tail935cc1.ts.net%3A8492' HORIZON_WEB_PROOF=/tmp/horizon-mobile-review.FT1JRF/ticks node tools/web_tick_check.cjs > /tmp/horizon-mobile-review.FT1JRF/ticks.log 2>&1
node /tmp/horizon-mobile-review.FT1JRF/followup-v3.cjs > /tmp/horizon-mobile-review.FT1JRF/followup-v3.log 2>&1
node /tmp/horizon-mobile-review.FT1JRF/startup.cjs > /tmp/horizon-mobile-review.FT1JRF/startup.log 2>&1
```

Harness limitations and corrected attempts: initial `focused.cjs` and `followup.cjs` uploaded before the import handler was visible and timed out; `focused-v2.cjs` used RAF-based polling while gating RAF and timed out. Neither failure is a product finding. v3 waits for the visible import UI and uses 20 ms interval polling. The first `trusted-focus.cjs` left Playwright focus emulation enabled and generated no blur events, so its successful movement was not a lifecycle test; v2 explicitly disables that emulation and records trusted events.

Unrun surfaces: actual phone, Safari, Duo GPU/folding/safe-area cutouts, real OS background suspension, native full test/parity suites, and another model family's review. No recommendation here treats desktop emulation as phone proof. All browser tests used the private served URL; the local URL was not separately exercised in this review.

Cleanup and source integrity: every owned Chrome session closed in `finally`; the temporary T3 preview tab was closed. No scratch servers were started. Existing PIDs 55502, 88366, 88504 and 9771 were still alive with their original commands at final verification; Tailscale configuration was untouched. The tested implementation was pinned at `1fc53d53515e41a1d9d90cbbe1dff7ed85d3490b`; `pin-final.json` captures an intermediate end-of-tests pin at that head. An external actor added untracked `tools/web_embed_check.cjs`, then advanced the shared branch to `2623ca8d875d7a5b81d0c17b61bae04699d3cd01` while this report was being finalized. The final checkout is clean and synchronized 0/0. The intervening commit changes audit files, status documentation, and the new embed probe, with no changes to the reviewed mobile implementation. This review is still limited to the requested range ending at `1fc53d535`; it is not a review of that later commit. `cleanup.json` and `pin-complete.json` record the final state. All external changes were left untouched.

Owned-texture screenshots/videos remain private under this temporary directory and are not embedded in the report.
