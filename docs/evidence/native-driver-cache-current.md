# Predictive driver cache equivalence

The private default driver now caches physical obstacle indices by path for one
immutable race setup. It also computes other actors' and traffic objects' future
locations once per command and lookahead time, rather than repeating the same
topology walk for every steering candidate. Each forecast is created lazily at
the existing twelve lookahead times. Cache ownership resets with the race.

Candidate order, scores, filters, signed arithmetic, crossing lateral estimates,
advance-error poses, normal controls and the sixty-tick prediction horizon are
unchanged. Opponents, physics and career rules were not modified. A public
regression exercises a traversal-bound error that mutates the returned road pose,
stationary crossing predictions, and an excluded actor with no rider state.

The independent oracle is the immutable command stream from the genuine current
career, captured before this optimization. Every optimized command matches at
all **536,874 snapshots**, across **31 races and all 25 course/level cases**.
The recorded original commands then advance the unchanged simulation; all 31
final canonical hashes still match. No second copy of the planning algorithm
supplies the expected commands.

Coverage includes 534,598 mounted ticks, 520 sliding, 540 getting up, 288 standing,
442 walking and 486 mounting, with six actual crash/recovery episodes. Strict
game/presentation Clippy and the public error-pose regression pass. The private
comparison runs at four workers; it is deterministic command validation, not a
frame-time measurement. Its first invocation used a relative fixture path and
failed before comparison because Cargo runs tests from the package directory;
the corrected absolute-path invocation passes and both logs remain preserved.

[Machine-readable receipt](native-driver-cache-current.json). Original setup and
command data remain private. Live frame-time acceptance and the replacement
candidate's hour-long soak remain separate gates.
