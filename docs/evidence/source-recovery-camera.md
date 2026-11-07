# Detached recovery camera

**69,120 actual native camera records pass**, covering 108 controlled 640-tick
sequences. The same frozen expected trace fails all 69,120 records before the
correction. [Aggregate receipt](source-recovery-camera.json). The previous 1,980
mounted-camera observations also pass unchanged; they are compatibility reruns,
not additional observations in this family.

## Corrected ownership

The native `Race::update_camera` takes longitudinal following from the player's
half mirror. That mirror follows the crash body while detached. Lateral following
and camera elevation still come from the quiet motorcycle: its absolute road
edges, lateral coordinate and ground word 0x24. Terminal/arrest states ramp lift
by 1 and pullback by 3 per camera update, capped at 300/900; both decay on release.
The host's bounded camera-back state therefore supplies lift as `back / 3`.

The render bridge previously replaced the player's motorcycle with its body
before computing the camera. As a result, the body's lateral position/ground
steered and lifted the camera, and terminal lift was absent. `CameraFollow` now
retains optional presentation inputs. A render clone captures the original quiet
bike location/lateral/ground before creating body/quiet-bike draw actors. Camera
x/height use those retained inputs; longitudinal follow and dynamic player depth
continue to use the body half. Already shifted pullback exports remain shifted
only once.

`Simulation::snapshot` also derives the quiet road's absolute centre from its
physical source-profile offset and relative ground edges. This remains available
when the motorcycle is on a different branch outside the body's visible route.
The average is `(2 * offset + left + right) / 2`, preserving native signed integer
rounding; averaging relative widths before adding an offset can differ by 1.
Visible-route and dual-road fallbacks use the same absolute-edge rule. New fields
are optional and are not part of authoritative/canonical bytes. The original
snapshot remains byte-for-value unchanged by each render comparison.

## Native matrix and public regressions

The producer calls the pinned, unmodified `Race::update_camera`, `camera_position`
and `camera` methods. It directly supplies independent quiet-bike, crash-actor and half-mirror
poses. 108 combinations cover three road offsets, two asymmetric edge sets, six
ground-height controls and three body/bike gaps. Each 640-tick sequence reaches
maximum lift/pullback under either the terminal latch or arrest flag, then fully
decays. Lateral fractions/signs alternate; quiet-bike ground varies around the
150-unit height clamp; the half and camera cross differently offset/width samples.
Both unshifted and already shifted host camera exports match every native row;
these two presentations of the same record count once.

Strict render all-target Clippy (`--no-deps`, warnings denied) passes; the parent
owns the final integrated dependency/workspace check. Four public regressions cover independent body/quiet inputs, another dual road,
an invisible quiet-bike branch, and the actual simulation snapshot's odd signed
centre rounding. Six existing offset/camera tests and the published historical
profile 2 replay hash pass. The initial new source-snapshot fixture omitted
required source-profile flags/constants; those invalid harness attempts remain
separate from the measured native baseline failure.

The native trace is `54d17c5ec95ce78a1bab244a1128cb580415efcc6031c3d706604f2f0741c1cf`.
The reference revision/library/EXE identities and dirty HUD-only paths are in the
receipt. No commercial bytes or frame traces are committed. Private final output:
`local/recovery-camera-body-pose-final/`; preserved before output:
`local/recovery-camera-before/`.

```sh
source local/reference-tools/env.sh
python3 tools/reference/check_recovery_camera.py --data local/cdrom/ROADRASH \
  --output local/NEW-recovery-camera
cargo test --target-dir ../target -p rr-render --test recovery_camera
```

This is a controlled camera callback/render bridge comparison. It does not
establish complete native variable scheduler, camera/outcome timing inside a full
race, moving physics, or TrackRing split/merge parity. Physical graph camera
ownership beyond the native Ring's supported traversal is a functional extension.
Complete changing-terrain recovery, repeated impacts and final graphical sequence
acceptance belong to the parent stage 4 integration.


## Retained arm during terminal pullback

A public whole-Simulation graph regression starts after an unequal merge on each
retained arm, applies a terminal state and checks the full900-unit pullback. The
version2 baseline incorrectly walked backward on the launch arm when the player
had arrived through the other arm. `scenery_prepare` now uses the followed body's
route context (or the quiet bike's while mounted) for backward topology movement.
Version1 is unchanged. Both arms now pass, including the exact remaining local
progress. Logs: `local/recovery-camera-merge-before.log` and
`local/recovery-camera-merge-fixed.log`. This is a functional graph correction,
not an additional native observation or native TrackRing graph comparison.
