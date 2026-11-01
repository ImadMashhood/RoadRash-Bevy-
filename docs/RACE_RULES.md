# Race lifecycle and immutable results

`rr-session` is the engine-independent boundary between a simulation and the frontend. `RaceSession` owns a bounded roster, session countdown, standings and a sealed `Summary`. The Bevy adapter owns pause/input, scene lifetime and presentation; it passes snapshots to the session after each permitted fixed tick. The physics/replay schemas 1–5 remain unchanged.

Research uses the pinned [race stepping](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race.cpp), [race screen](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/race_screen.cpp), [lifecycle countdown](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/lifecycle.hpp), [session lifecycle](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/session.cpp) and [career result boundary](https://github.com/arkits/road-rash/blob/7aecf6044f2dd773b925b325823f30d648d61c89/src/career.hpp). No original executable is run.

## Timing and outcomes

The countdown lasts 180 fixed session ticks (three seconds at 60 Hz), consistent with the pinned countdown duration. During these ticks the modern policy freezes the entire simulation, including opponents, traffic and RNG. Native countdown processing can still think other actors; full scheduler parity is not claimed. Held driving input first reaches simulation on session tick 181. Pause stops fixed session ticks as well as physics. Restart rebuilds the setup, clears pending commands, resets the same seed, restores the full countdown and discards the abandoned result. A career restart consumes a fresh monotonic race ID without paying or charging for the abandoned run.

Finish, wreck and arrest come from the existing simulation terminal transaction. Exact-boundary/overshoot detection and `Arrested > Finished > Wrecked` priority remain there. A result is created only once after the local player terminates. Its names, positions, statuses, elapsed race ticks and normalized remaining bike condition are owned values, independent of later live entities. Integer finish tick sorts first; racers finishing in the same tick sort by stable ActorId. No sub-tick order is invented from already-clamped positions. `Summary::validate` checks roster identity, positions, bounds and win/place consistency before a career settlement.

| Simulation/session event | Result | Frontend | Big Game consequence |
| --- | --- | --- | --- |
| Finished, place 1–3 | Won | Results → standings/continue | Prize by place × level; qualify course |
| Finished, place 4–15 | Lost | Results → standings/continue | Prize by place × level |
| Arrested | Arrested | Results → standings/continue | Level × $400 fine |
| Wrecked | Wrecked | Results → standings/continue | Bike price / 10 repair charge |
| Confirmed quit | Quit | Results → continue | No prize/fine/unlock |
| Optional configured timeout | TimedOut | Results → continue | No prize/fine/unlock |

No timeout is enabled by default: an original race time limit has not been evidenced. Quit/timeout financial behavior is an explicit modern policy. If a fine/mandatory wreck repair is unaffordable, the pinned career API resets character cash, bike, level and qualification; the settlement reports game over. The source top-three qualification rule and money APIs are separately verified against the pinned native port, not inferred from screenshots.

## Progress and ownership

Selected-route paths retain their canonical offsets. Alternate branches use topology portals to locate preceding and following selected-route anchors, then normalize the alternate segment's traveled fraction into the interval between those anchors. A 400-unit alternate branch halfway between the same junctions ranks equally to a 100-unit branch halfway through; it does not gain four times the progress. Intermediate alternate paths use bounded shortest-distance anchor walks. Disconnected or invalid paths have unknown progress and cannot borrow another road's local coordinates. This normalized race-ranking policy is explicit; native scan-list order equivalence is unmeasured.

Finishers precede active racers; active racers precede failed racers. Comparable progress descends, with ActorId as the final stable tie. Crashed but recovering racers remain active. Results freeze the field when the local race ends, so unfinished rivals are labeled unfinished rather than awarded fictional finish times. A single-player result does not wait for the whole field.

On leaving results, quitting to the main menu or returning from a race, the adapter drops the source scene and all simulation actors, road users, recovery state, contacts and queues; it publishes empty snapshots and reconciles siren ownership to empty. One persistent presentation camera/surface is reused, with the source surface hidden when no source scene exists. Repeated factories verify identical initialization hashes, stock rival condition and dropped scene ownership. This is an allocation/ownership check, not a claim that the system allocator returns identical RSS after every race.

## Reproduction

```bash
cargo test --locked -p rr-session
cargo test --locked --release -p rr-game -p rr-presentation
cargo run --locked --release -p rr-session --bin rr-race-check -- \
  docs/fixtures/race-rules-replay.json irregular /tmp/race-rules-new.json
# Exercise the private source factory across all five courses and three bike choices:
RR_SESSION_DATA="$PWD/local/cdrom/ROADRASH" RR_SESSION_CATALOG="$PWD/local/catalog" \
  cargo test --locked --release -p rr-game source_factory_repeated
```

The authored race fixture, session-level 30/60/144/irregular and effects on/off tests, legacy replay checks and actual UI captures are distinct evidence. [Part 15 receipts](evidence/part-15.md), [frontend/economy contract](FRONTEND.md).
