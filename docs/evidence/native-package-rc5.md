# Replacement candidate package verification

The frozen source is `24d05dcf399035fe6c6e70bb19fe1b12488c1cec`.
A fresh detached checkout rebuilt all eleven workspace crates with locked
dependencies, reusing the existing external dependency artifacts. All five
checked executable hashes match the preserved frozen binaries exactly, including
game `55051bf76fcac2225bac25d24bf24863f2ea0f2c7a5357d982daa9d148fee922`.

Two independent normalized local packages are byte-identical:
`a58ea675286babc9b030a7164698769f374546ba358b88c7aee29a8db8e36eeb`.
Every one of the 712 manifest entries passed size/hash verification. All 369
dependency notice records contain their listed license texts; the embedded font
OFL is present. The archive contains application binaries, launcher, guide and
notices, with no original or converted game media.
[Local reproduction receipt](native-repro-rc5.json).

The local package also passed an actual read-only Bubblewrap launch with the
checkout hidden and Cargo absent from PATH. A fresh own-ISO import validated 341
files and built the catalog, then the actual app opened for 120 frames and exited
normally. Missing-disc guidance returned the expected failure. An unchanged
ordinary-keyboard profile reopened first under RC4, then under the replacement
application. All three profile JSON files, the active import pointer and all
6,486 imported/catalog files retained their exact hashes through replacement and
deleting only the disposable application. Original ISO and profile were preserved.
[Local sandbox receipt](native-package-local-rc5.json).

The [actual Ubuntu CI download](native-download-rc5.json) also passes its published
checksum and all 712 manifest checks. Its archive SHA256 is
`6656faaed67e79fb2d54898513806ce4a522efd48898028851781edc9c5cd910`;
its game is `4520d5a27a77ba5cbfadf2f2a7045dc206557dbf73be378eb47e67daea499f21`.
These are deliberately distinguished from the Fedora bytes above. An unchanged
read-only copy lives outside the host checkout. The downloaded application passed
the same [source-hidden own-disc/upgrade/uninstall checks](native-package-sandbox-rc5.json),
including all 6,486 retained import/catalog files and three profile records.

The [downloaded application keyboard walkthrough](native-packaged-ui-rc5.json)
used ordinary XTest events in the actual Bevy window, with no application action
API. A deliberately altered cash value with an unchanged checksum was visibly
rejected. Settings → Restore Career Backup restored the exact original file and
retained the damaged primary separately; the original source profile was untouched.
The same application passed real Thrash driving, held-control resume rejection,
neutral resume with warning clearing, actual quit results, normal close/reopen,
showroom preview and the full purchase quote. Confirming an unaffordable purchase
preserved the original $200 career. French, 125% text, high contrast and the active
save slot survived reopening.

The curated commercial captures remain private under `local/rc5-packaged-ui/`.
Use 00/03/04/06/07/09/14/15/16/17/18; misnavigated captures 01 and 10–12 are excluded,
and 13 was not captured after a normal application exit. Heading accent/top-stroke
contact near the fixed upper separator was reviewed as a minor cosmetic
observation: the title, actions, notice and footer remain readable. Pixel-perfect
original menu layout is not claimed. Audio/videos were off, and this does not
certify a physical controller or a fresh graphical file-picker interaction.

Genuine downloaded-build career, performance and owner G5 review remain separate
requirements. Provisional source builds and failed
performance/lifecycle probes remain preserved privately; they are not relabeled
as replacement release acceptance. Raw commercial data and captures remain private.
