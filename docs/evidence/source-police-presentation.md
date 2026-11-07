# Police rider presentation

The native asset comparison passes all **90 body/shadow frame records**: both
45-frame sheets, decoded RGBA pixels, logical sizes and registration points.
The actual pinned high-resolution loader selects `LOCOP.DAT` when `HICOP.DAT`
is absent and uses `HISHADOC.DAT` for its shadow. Cop bank1 uses palette ID0xde,
shadow bank3 uses0xdf, with original palette indices and no ordinary-rider remap.
[Receipt](source-police-presentation.json) retains the source/library/native trace
hashes. Private trace: `local/police-assets-first`; reproduce with
`tools/reference/check_police_assets.py` after sourcing the reference environment.

The renderer now selects this separate body/shadow asset bank for a shared
encounter unit tagged `police`, including its linked detached rider/quiet bike.
A linked population mirror supplies siren/lifecycle metadata without drawing a
second cop. Empty cop frames do not acquire generic rider pixels. The public
colored-asset regression requires cop pixels, detects an unlinked legacy draw,
and verifies that the linked mirror leaves the image byte-identical.

Frontend roster construction and the fallback HUD rank/field count exclude
police. The race session ranks its fixed initial racer roster; dynamic police
cannot steal a finishing place. This is presentation/ranking evidence, not yet
complete police combat, recovery, arrest or live visual-sequence acceptance.

## Cop recovery display adaptation

The pinned `race.cpp` advances cop crash entry, rider/bike recovery and remount
using normal bank0. Its `Game::rider_nodes` later indexes cop bank1. Real bank1
has no codes29/34/35/38/41/44. This is a reference rendering gap; no complete
native cop crash raster comparison is claimed. Recovery authority retains the
measured bank0 clock. A disposable render copy resolves falling-body and quiet-bike clocks through
bank1. Upright recovery needs a separate adaptation: all 45 LOCOP slots were
inspected (33 nonempty); they contain mounted, falling and prone poses, with no
upright foot sprites. Bank1 code37 selects prone frame42, walking38 falls back to
37, and mounting41 falls back to33. That produced a visibly rolling cop during
standing/walking/mounting, despite matching authority traces. The rejected
sequence is preserved at `local/source-police-art-live-final`.

Selected body clips36–41 now use the original HIBOB/HISHADOW foot artwork and
its own RRAN logical sizes, pivots and shadows. Original rider palette6 supplies
a restrained dark/light outfit; no existing rider remap exactly matches the cop
uniform. Mounted, sliding and quiet-bike poses retain the cop artwork. No extra
sprite-size multiplier or second animation clock is introduced. The normal-bank
selected frame and every authoritative field remain unchanged. This is a
functional source-art adaptation, not native cop raster identity.

The colored-bank regression covers all six foot clips, unchanged snapshots,
separate mounted cop pixels, and duplicate-draw suppression. The final 900-tick sequence reaches sliding400, get-up447, standing537, walking585,
mounting612 and mounted693. Four replay schedules and the release game-window
replay agree on hash `a42ea7133b4bd90c60f39e5710cc9c7ef22fb9a136c9f26fd6a551547a04f9bb`.
Get-up480, walk585, mount612 and remounted693 images were inspected; the actual
game-window walk capture also passes. Private receipt: `local/source-police-art-foot-fixed/comparison.json`,
graphical receipt: `local/source-police-art-foot-fixed/gpu.json`. The mounting
source frame includes the standard rider bike, so cop uniform/bike identity during
this upright fallback remains an explicit visual limitation.


The current recovery integration supersedes the stage3 private replay hash above.
Final palette-correct receipt: `local/source-police-art-palette-final/comparison.json`;
current game-window replay: `local/source-police-art-recovery-current/gpu.json`,
hash `3b9833b21edffb920a823d6eb27cfcc07770854ca2eaee6df3597d061575ccb6`.
