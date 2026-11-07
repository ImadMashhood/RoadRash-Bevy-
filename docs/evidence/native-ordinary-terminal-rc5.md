# RC5 ordinary keyboard route: release-blocking failure

**This route failed. RC5 has no passing keyboard recovery acceptance.** The actual
downloaded application is source `24d05dcf399035fe6c6e70bb19fe1b12488c1cec`, game SHA256
`4520d5a27a77ba5cbfadf2f2a7045dc206557dbf73be378eb47e67daea499f21`.

All controls came from ordinary XTest keyboard events in the real application,
with a fresh profile, read-only app and hidden source checkout. No HP, money,
position or terminal state was edited; no MPC, test-control API or assist was used.
Changing the Thrash level used the normal Restroom menu.

The first CITY level-one attempt naturally reached **BUSTED**, place 15, displayed
time 76.62 seconds, then closed normally. Its optional input observer did not
export a file; that missing observer is excluded and no replay hash is claimed.

A second CANYN level-one race also naturally reached BUSTED at 76.62 seconds.
Continuing through its normal results screen exported **1,515 live commands**.
The preserved recording passes independent 30/60/144/irregular replay: 4,597 ticks,
canonical hash `c880d51fd310dfdfb0cbc231d5d9b33421ea9f0c3f68449363a2e87e743d7d5d`,
natural arrested outcome, no assist and incident count zero. Forbidden injected
damage/impact/terminal/weapon/pose/fork controls are absent. This proves the natural
terminal flow, not crash recovery.

The next race used ordinary menu choices to select level five, Axle and CITY.
Throttle, steering and leaning produced unclassified hidden-rider frames followed
by a mounted rider. Those captures are not accepted as crash/recovery proof.
After braking near standstill, pressing the normal **V** dismount key caused:

```text
thread 'Compute Task Pool (2)' panicked at crates/simulation/src/encounter.rs:1358:49:
no entry found for key
```

The actual application exited **101** at 14:39:33 UTC on 2026-11-07, before the
requested dismount capture. The last recorded XTest sequence was Down held for
one second, released, then V. Its event-log timestamp records key-action completion,
which follows the application's panic. The third race's passive recording was not
exported; the preceding genuine CANYN record remains unchanged.

Raw events, logs, captures and recording hashes remain private under
`local/ordinary-terminal-rc5/` and `local/ordinary-recovery-rc5/`. The exact selected
bike ID and physical fork half at failure were not independently observed and are
not invented. A corrected immutable candidate must verify ordinary voluntary
dismount, walking and remounting. No recovery or owner G5 acceptance is signed.
[Machine-readable failed receipt](native-ordinary-terminal-rc5.json).
