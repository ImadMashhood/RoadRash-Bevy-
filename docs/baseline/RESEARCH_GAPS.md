# Evidence gaps and original-game capture protocol

Phase 01 remains open. Original-game execution has not been performed in this chat.

1. Capture every original menu item and navigation/cancel transition. The upstream screen constant inventory is only a starting index; native convenience screens must be identified separately.
2. Measure original keyboard and joystick bindings and held/edge-trigger behavior. Upstream convenience bindings are not proof of original defaults.
3. Verify in-game bike names, character display names and their selection/unlock conditions. Asset stems are not display strings.
4. Measure viewport/camera modes, sprite scale, race tick timing and pause semantics.
5. Capture all course/level combinations, progression, money, repair and purchase rules.
6. Separate reproducible original defects from native-port limitations and desired improvements.

For each capture: record executable hash, OS/Wine or hardware environment, game configuration, initial save hash, course/level/bike/character IDs, input timestamps and timebase, seed if measurable, video/trace hashes and exact observation. Keep captures local and commit metadata only. Repeat measurements before assigning exact timing. Never invent a seed or claim determinism from a video alone.

Confidence labels: `verified-local` (reproduced here), `upstream-reported` (pinned source/document), `inferred` (reasoned interpretation), `unknown` (measurement required). Bugs require trigger, expected/observed result and evidence source; open unknowns are not automatically defects.
