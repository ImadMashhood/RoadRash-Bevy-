# Recovery camera and guardrail draw window

All **2,520 native sub-slice queries match**. Each native record is checked against
two Rust representations of the same camera: an unshifted frame-follow coordinate
and an already shifted completed/pending coordinate. Those duplicate Rust forms
are not added to the native observation count. [Receipt](source-recovery-rail.json).

The actual pinned `Race::sub_slice` packs a rail's drawn height into its flag byte
only through camera sample+31. The version2 baseline double-subtracted terminal
pullback when `pending_camera` already held the shifted coordinate, producing
252 valid differences against the same frozen native expected trace. Contact-walk
version2 now subtracts pullback only from the unshifted frame-follow form.
Version1 retains its historical behavior. The public regression checks the strict
inside/outside edge, equal pending/frame-follow results and frozen version1 output.

The native matrix covers early-course clamping, fractional positions, pullbacks
0/3/6/255/768/897/900, samples immediately before/at/after the draw limit, all low
flag-bit combinations and drawn heights4/128/252/256 including byte wrapping.
This is a controlled query-kernel comparison with the actual C++ library; it does
not establish complete Ring scheduling or full vehicle collision trajectories.
Private traces remain under ignored `local/recovery-rail-*`. The first producer
attempt failed C++ warnings-as-errors compilation and is invalid harness history.

Reproduce after sourcing `local/reference-tools/env.sh`:

```sh
python3 tools/reference/check_recovery_rail.py --data local/cdrom/ROADRASH --output local/NEW-recovery-rail
```
