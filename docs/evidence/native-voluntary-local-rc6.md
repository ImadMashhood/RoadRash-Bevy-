# Local dismount correction: ordinary-keyboard preflight

**Passed locally; not delivered RC6 acceptance.** The read-only external bundle
contains RC5 support files with only its game replaced by the corrected local
executable `666d897859240b9e929f69d2853d4fcef7513937d30a35359604b0dd523ad683`.
Its package manifest remains RC5. No published artifact is silently relabeled.

Ordinary XTest keys selected Thrash/CITY, performed V at standstill, supplied
brief Left/Up walking input, remounted and resumed Up driving. CITY level one
shows a prone rider/fallen bike at 0.5 seconds, an on-foot rider at two seconds
and mounted riding at four/eight seconds. The normal Restroom menu then selected
level five (captured separately), followed by CITY. **Two successive level-five
V/recovery/remount episodes passed**, including resumed driving, without a panic.
Normal UI Quit Results → Continue → Courses exported the observer recording;
window close exited zero. Quit is not counted as a natural full race.

All three exported live recordings pass 30/60/144/irregular schedules: **12
independent replays**, with no injected damage, impact, terminal, weapon, pose or
fork controls. The confirmed CITY level-five recording has 1,506 ticks, two
recovery incidents, a final mounted state and canonical hash
`4077aef4526ef469e94fd10e64dfbf4aee8a8c8306933c3639cab0fbf3c16a0a`.
The earlier CITY level-one run also naturally reached Arrested after one voluntary
recovery incident; its 4,593-tick hash is
`283186c1a726fd6cd65daff78e5d8870819f612148bc15b8ab6da5a9f0e2c8e6`.

This demonstrates **voluntary recovery**, not an impact-induced crash. No profile
HP, money, position, assist or runtime state was edited; no MPC or game-control
API drove the application. The old RC5 panic remains preserved as a release failure.

Curated private evidence under `local/local-fixed-v-preflight-rc6/`:

- `l1-valid-first/v1-0.50s.png`, `v1-2.00s.png`, `v1-4.00s.png`.
- `07-confirmed-level-five.png`, `08-confirmed-city-l5-countdown.png`.
- `city-l5-valid-first/v4-*` and `city-l5-valid-repeat/v5-*`.
- `10-city-l5-resumed-repeat.png`, `12-after-continue.png`.

An initial startup-window race produced a menu-only burst and is excluded.
`l1-repeat` occurred after a natural arrest and is excluded as repeated-V proof.
The earlier folder named `l5-first/v3-*` actually belongs to HIWAY level one after
misnavigation; it is not CITY level-five evidence. The later confirmed CITY cases
above supply that proof. [Machine-readable local receipt](native-voluntary-local-rc6.json).
