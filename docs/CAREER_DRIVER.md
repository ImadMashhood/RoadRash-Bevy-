# Private native career driver

`apps/game/src/acceptance.rs` exercises the normal native simulation, session,
career, shop, social observation and save code. It starts a fresh character and
uses throttle, brake, steering, nitro and ordinary combat inputs. Terminal state,
money, inventory, AI, contacts and qualifications are never injected. Recovery
uses the native automatic return/remount with no recovery assists. Each race
requires all 15 source racers and native police.

The driver is an unattended test pilot, not an alternative player controller.
Its route cache follows the current physical source-road profile after splits and
merges. Planning uses the current physical ground and source building offsets.
Vehicle predictions reserve lateral space for the normal 1024-per-tick lane
movement and a longitudinal window large enough for the five-tick planning
samples. Source collision behavior remains unchanged.

Bike purchases and repairs are ordinary `Career` transactions. Early levels can
repeat MEDLY to earn the planned upgrade; levels four and five attempt remaining
qualifications with the currently owned bike rather than requiring a specific
purchase before trying another course. A failed race retains its ordinary repair
cost, fine, condition loss or game-over result. A bounded or failed campaign is
not completion evidence.

Every attempt records commands, source setup, result hash, original condition
capacity, initial/final condition percentages, pre-shop career, actual shop
receipts, settlement and checksummed save. The social settlement helper is shared
with the ordinary frontend, so saves include relationships observed during real
race ticks. Save decoding runs after every settlement.

Use a new private output directory for each run:

```sh
CARGO_TARGET_DIR=../target cargo build --release -p rr-game
RR_ACCEPTANCE_ATTEMPTS=200 ../target/release/rr-game \
  --install-root local/game-install --catalog local/catalog \
  --functional-check local/career-current-car-safe-full --audio off --videos off
```

Capture the actual running executable before rebuilding, preserve its SHA256 and
keep all source-containing recordings local. A genuine completion must then pass
[the independent campaign audit](CAMPAIGN_VERIFIER.md), including all four replay
schedules, the full financial/save ledger and all 25 course/level cases. This
command driver does not replace the separate ordinary keyboard player walkthrough.

The ignored `recorded_career_incidents` test is a diagnostic for an existing real
recording. It replays the recorded controls unchanged, prints player crash context
and asserts the original final state hash. Select a local recording with
`RR_CAREER_REPLAY` and run the test in release mode; it does not create career
acceptance evidence.

The [current completed career](evidence/native-career-current.md) and its
independent receipt certify the frozen source profile described above.
