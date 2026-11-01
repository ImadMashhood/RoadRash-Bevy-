# Owner scope decision: original-runtime comparison removed

The owner explicitly removed the requirement to run the original executable or compare the recreation against its runtime captures. This decision supersedes earlier README and status requirements for a working original installation, Wine/Proton setup, original-menu walkthroughs, original input traces and screenshot parity.

Retired checklist gates: 01.02.b, 01.03.b, 01.04.b, 01.06.b, 08.01.b, 15.12.b, and all of task 11.12. Other IDs remain stable. Part 01 uses documented static inventories and current-engine smoke/fixture evidence. Unknown original handler behavior stays unknown and is addressed as the relevant gameplay/frontend feature is implemented; it no longer blocks the foundation.

Still required: supported disc hashes, file provenance, bounded parsing, preserved palette indices/remaps, explicit arithmetic semantics, reproducible content builds, tests of intended engine behavior, replay/state determinism and internal screenshot regressions. Native/enhanced texture switching is tested against this engine's replay state. No claim of original runtime parity is made.

Historical receipts describing failed Proton launch or incomplete original captures remain research history. They impose no ongoing task or release prerequisite. 3D remains deferred.
