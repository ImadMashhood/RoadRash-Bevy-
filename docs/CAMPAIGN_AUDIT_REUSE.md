# Exact-byte career audit reuse

An actual downloaded game must still complete its own fresh genuine career.
Independent replay results can be reused only when every authority input is
identical to an already accepted complete source audit. This is a content-identity
proof, not another engine execution or a shortcut for a changed case.

The trust anchor is the accepted public source receipt and its externally pinned
SHA256. That receipt must contain a successful 31-attempt/25-case/124-run audit,
all four schedules and every checkpoint/financial/save gate. The original raw
audit output, source campaign, recorded replays and saves remain private and
immutable. The checker verifies their digests against that accepted receipt.

`tools/verification/campaign_reuse.py` then requires:

- A separately completed campaign in a different output directory. Raw campaign
  bytes must match after replacing only the 31 nonnegative integer `wall_ms`
  values. JSON duplicate keys, altered numeric encodings, added fields or any
  other byte change reject reuse.
- All 31 replay files, all 31 per-race saves and the final save must match the
  audited source hashes exactly.
- The exact independent replay binary and all four audit dependencies must match:
  `campaign.py`, the PE-reading helper, the required-file manifest and selectable
  actor metadata. The pinned executable supplies unchanged prices/prizes.
- The separate original and fresh downloaded imports must each contain all 341
  original files at their expected sizes/hashes. Symlinked source files reject
  reuse. The source executable identity must agree with the accepted audit.

Example after a fresh actual RC6 career, substituting its verified imported data:

```sh
python3 tools/verification/campaign_reuse.py \
  --reference docs/evidence/native-career-source-rc6.json \
  --reference-sha256 153525ebf877e4338da6971b09797d7dfdf715420ccf1e9a04ae68389485221b \
  --source-audit local/career-source-rc6-audit/receipt.json \
  --source-campaign local/career-source-rc6/campaign.json \
  --actual-campaign local/career-downloaded-rc6/campaign.json \
  --source-data local/rc5-package-download-sandbox/user-data/runtime/import/installs/install-1U0jqv/ROADRASH \
  --actual-data "$RC6_VERIFIED_DATA" \
  --binary local/career-source-rc6/rr-replay-companion \
  --repo . --output local/career-downloaded-rc6/exact-audit-reuse.json
```

The receipt explicitly reports **0 new independent replay executions** and
**124 reused source executions** for 31 byte-identical inputs. It records the
reference receipt, raw source audit, each input/save, dependency and checker
hash. It never relabels the original source executions as a second downloaded
audit. Package provenance, the fresh actual game execution, manual-V recovery,
graphical finale, performance/hour soak and owner G5 remain separate evidence.

Any mismatch fails closed. Preserve the discrepancy and run a new independent
audit for the affected inputs; changed dependencies or shared tables require
the complete audit. Do not update expected hashes to conceal a failed comparison.

Eight synthetic regression tests exercise permitted wall-time changes, strict
authority encodings, corruption, partial audits, wrong profiles, dependencies,
source files, symlinks and accidental reuse of the same directory. They run in
normal and optimized Python modes. A private historical-input native smoke
exercises all 31 input pairs and both complete source inventories; it is checker
validation, not actual downloaded RC6 acceptance.
