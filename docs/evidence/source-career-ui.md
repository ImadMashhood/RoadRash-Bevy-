# Current ordinary keyboard walkthrough

The actual Rust Bevy application passed the ordinary keyboard route using an
isolated X11 window. Input came through XTest key events; screenshots came from
that window. Application action APIs were not used for this walkthrough.

The route covered a new Big Game, rider selection, club dialogue, course launch,
driving, pause, held-control resume rejection, neutral resume, real quit results,
standings, showroom specifications, a priced purchase confirmation and denied
purchase, manual save, normal close, reopen and load. The career retained level 1,
$200 and full condition. French, 125% text, high contrast and reduced motion
survived restart. Both saved records passed checksum validation.

The walkthrough exposed stale notices and a missing price breakdown. Successful
navigation now clears messages from the previous context; purchase and repair
confirmations show costs and available cash. Successful neutral resume clears
the held-control warning. Fifteen frontend regressions pass, with five explicit
private-data skips. A separate actual-data factory audit confirms original bike
condition capacity for 15 course/bike/carried-damage combinations.

Private captures, event chronology and file hashes are retained under
`local/career-keyboard-walkthrough/receipt.json`. Misnavigated screenshots 19 and
33–35 are excluded; corrected resume captures 39–40 show rejection and clearing.
The physical desktop was unmapped, so this used a private virtual X11 display.
Audio/video were disabled; this does not certify controller hardware or media.
Commercial screenshots remain private. This is source-tree UI evidence; the
replacement delivered package still requires its separate release checks.

The genuine completed career was also copied unchanged to a separate profile and
opened through Main → Resume Big Game → Club → Restroom. The original menu showed
player level 5. The copied save still decodes exactly as the preserved audited
original: completed, $12,797, bike 8 and 98% condition. Normal window close exited 0.
Private evidence: `local/career-keyboard-finale/receipt.json`.

Machine-readable scope: [source-career-ui.json](source-career-ui.json).
