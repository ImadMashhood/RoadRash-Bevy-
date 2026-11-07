# Current career audit

`tools/verification/campaign.py` audits private native career artifacts. It does not
create controls, alter native physics, inject results, or reduce replay checkpoint
frequency. Keep the source installation, replay JSONs, and raw native reports local.
Only the aggregate `receipt.json` is suitable for publication.

Run from the repository root against a frozen completed campaign and replay binary:

```sh
python tools/verification/campaign.py \
  local/career-current-car-safe-full/campaign.json local/career-full-audit \
  --data local/cdrom/ROADRASH --binary ../target/release/rr-replay --jobs 12
```

The output directory must be new. `--jobs` accepts 1–12 workers. One shared executor
prefetches at most `ceil(jobs / 4)` races, capped at three; every race still runs at
30, 60, 144, and irregular schedules. Report paths include both attempt and rate.
Every final hash and complete array of canonical checkpoints is checked, including
checkpoint cadence. On failure queued work is cancelled and running work is joined;
temporary reports are removed. The auditor parses only the current replay JSON,
rather than retaining the whole campaign's large replay bodies in memory.

The audit checks the current combat/recovery/contact profile (4/2/2), 257 source
angles, exactly actors 1–15, police, disabled recovery assists, and absence of injected
state commands. It pins the source executable SHA before reading prices and prizes.
It checks SaveSchema 2 body SHA256 (the save checksum is SHA256, not CRC32), the
fresh-character ledger, purchase/trade-in/repair receipts, initial and actual final
bike condition, settlement prizes/penalties/game-over boundaries, progression, and
all 25 course/level cases. The final receipt records script, replay binary, source
executable, replay, state, and save-body digests plus audit/profile/source metadata.

`--partial` accepts an explicit progress snapshot or incomplete campaign and emits
`partial_audit_passed`. It never emits a completed-campaign pass. Existing historical
receipts are evidence of their recorded profile only.

Regression checks use synthetic public fixtures and need no source installation:

```sh
python -m unittest discover -s tools/verification -p test_campaign.py -v
python -O -m unittest discover -s tools/verification -p test_campaign.py
```

Native integration smoke checks should use a private two-race manifest beside the
original replays and saves, with `career` copied from the second saved state, and
`--partial --jobs 8`. This exercises real concurrent replay without representing
the smoke receipt as full-campaign evidence.
