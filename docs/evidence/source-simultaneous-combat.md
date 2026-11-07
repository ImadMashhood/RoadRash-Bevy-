# Simultaneous source combat kernel, profile 4

**64,512 complete field-tick records match**, covering **322,560 actor-ticks** in
672 controlled three-, four- and eight-rider trajectories. The shared MSVC RNG is
anchored once per trajectory, then free-runs through every actor/tick. These are
two ways of counting the same records, not additive observation totals.
[Receipt, identities and preserved failures](source-simultaneous-combat.json).

The matching native expected trace is identical before and after the correction.
Historical profile 3 fails 52,917 of these 64,512 field ticks; the first divergence
is tick 15, before any crash. An earlier actor hits a rider which then starts its
held attack during its own think. The historical batch-initiation/fixed-stun path
instead leaves that rider in reaction animation 9. Profile 3 remains available
unchanged for old recordings; this correction is explicit combat profile 4.

## What profile 4 corrects

- Initiate, resolve and advance each rider's fight transaction in actor order.
  Native reaction clip 9 preserves its prior queue; held input can start a new
  attack on that rider's own think rather than waiting for an authored stun timer.
- Separate windup acquisition from impact acquisition. Impact uses the live
  position-sorted half list, including detached halves, with backwards ties and
  stable previous list order when actors converge to equal positions. Windup
  retains the source's first-created-rider tie rule.
- Read actual retained clip side/backhand flags. The kick table aliases its
  normal/backhand clips, so native live kick transactions also transfer signed
  300 speed. The old command-kind test omitted that behavior.
- Preserve the second non-player chain-failure RNG draw and resolve animation 6
  through the attacker's own bank/fallback table before queueing it.
- Use each actor's animation configuration. A fallback which is still a valid
  attack for the latched weapon can hit; a fallback belonging to another weapon
  ends the attack. A successful theft which selects a valid new-weapon attack
  continues that clip/clock instead of restarting from held input.
- Keep weapon ownership atomic, reject stale/missing targets, retain the source
  response against a detached rider's owning bike/stamina, and apply relationship
  callbacks immediately after landed damage. Police AI owns its regeneration;
  the shared kernel resolves/advances a scheduled cop attack once.

## Controlled coverage

Each family has 84 trajectories and 8,064 field ticks, mirrored on both sides,
with fists, club, chain, kick and every supported backhand command. Two initial
RNG seeds cover different theft/chain outcomes.

| Family | What changes during a trajectory |
| --- | --- |
| Held simultaneous commands | Every rider can attack, interrupt and react |
| Equal positions | Live list predecessor versus initial acquisition ties |
| Reach entry/exit | Strict longitudinal/height boundaries during windup |
| Retargeting | The nearest half moves beyond the acquisition window |
| Contested theft | Multiple unarmed riders contest one attacking club owner |
| Bank 1 | A controlled non-AI rider uses police clips and exact fallbacks |
| Depletion | Rider/bike separation and subsequent detached-half targeting |
| Converging positions | Physical list order survives later equal positions |

The producer installs authored poses, refreshes native `sort_schedule`, then
calls actual `fight` and `advance_ride_animation` per actor. Mounted half mirrors
are installed explicitly; `sync_half_mirror` is not misused to copy a dormant crash
body into a mounted rider. Detached mirrors follow the actual crash body. Physics,
crash-motion callbacks and AI decisions are deliberately not scheduled by this
kernel probe. The legacy adapter freezes already detached bodies while using its
historical movement-based initiation API, so it observes the same callback scope.

Every tick compares stamina maximum/current, condition, signed speed, lean,
immunity/theft deadlines, carried weapon, animation/frame/queue/loop state,
detached-body presence and selected-animation presence, plus shared RNG state.
Inventory validation runs every Rust tick. Native first-crash observation confirms
animation 35 with neither body nor quiet-bike descriptor selected yet; an initial
legacy frame number is not evidence that an upright rider should be drawn there.

## Regression and provenance checks

Sixteen public authored kernel/bridge tests cover list/history ties, detached identity,
second RNG draw, kick clip aliasing, held-command interruption, actor/weapon
removal, bank fallbacks and theft clock continuity. Six of these also check physical-road acquisition/impact and external fallback. Version validation now accepts
0–4 and rejects 5; the published historical replay hash still passes. The original
53,760 two-rider profile-3 chain observations remain byte-for-value unchanged.
That legacy oracle retains its original reanchoring policy and is only a
compatibility check here; it is not substituted for the new free-run proof.

Strict simulation Clippy, Rust formatting/diff checks and C++ warnings-as-errors
pass. Two early full-matrix retargeting experiments used stale native sort keys
and are explicitly invalid for acceptance; incomplete bank-fallback writer runs
are retained separately. Neither appears in the passing count above.

```sh
source local/reference-tools/env.sh
python3 tools/reference/check_combat_simultaneous.py --data local/cdrom/ROADRASH \
  --output local/NEW-simultaneous --profile 4
cargo test --locked --target-dir ../target -p rr-simulation --lib source4_
cargo clippy --locked --target-dir ../target -p rr-simulation --all-targets -- -D warnings
```

The private accepted output is `local/simultaneous-physical-final/`; the valid paired
profile-3 failure is `local/simultaneous-sorted-before/`. Expected trace SHA-256 is
recorded in the receipt. Base revision is `a5eff4c` plus this working tree, verified
2026-11-07. Commercial assets and detailed traces remain private.

## Physical-road bridge follow-up

Combat profile 4 now rebases windup and impact candidates through the same
`Roads::physical_delta` used by source collision contacts. This admits same-path
and directly adjacent seam contacts, rejects opposite physical fork masks and
nonadjacent/disconnected paths, and keeps a detached rider on its own physical
route independently of its quiet bike. External road-user fallback uses local
physical positions, including paths absent from the selected route projection.

Unequal branch histories cannot provide one global distance axis. Each observer
therefore retains a stable half-list history, sorting comparable halves by their
rebased physical distance while retaining historical tie order. Actor removal
cleans its own view and every other view. Empty history remains absent from old
serialized states. This is an explicit authored graph policy, not a comparison
with native TrackRing graph scheduling.

Six authored tests pass. Four paired ordinary `Simulation::advance` trajectories
run 96 ticks each and match an equivalent straight-road transaction, including
shared road after unequal branch histories, forward/reverse seams and three-actor
zero-gap ties. Other checks cover convergence history, bilateral fork exclusion,
nonadjacent/disconnected paths, independent detached-body ownership and external
side fallback. The final native kernel rerun retains exactly the same trace hashes
and all 64,512 matching field ticks. The historical published replay hash also
passes after the bridge and API split. The initial invalid zero-steer-scale
authored fixture is retained separately and is not counted as a runtime failure.

## Limits and remaining stage-3 work

This proves the controlled live-list combat kernel, not the entire native
scheduler, AI or physics. The bank-1 fixture uses `ai=false` and does not establish
police AI parity. The authored physical-road bridge is tested functionally, while native
TrackRing graph scheduling remains outside this controlled oracle. All-bike handling/landing and
police lifecycle/AI have separate evidence. No complete career, replacement
performance/package release or project-owner acceptance is claimed here.
