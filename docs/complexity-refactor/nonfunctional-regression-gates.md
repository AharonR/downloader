# Non-Functional Regression Gates

Date: 2026-02-19

This phase adds explicit, repeatable checks for performance and operational stability during refactor phases.

## Gate suite

Run:

```bash
cargo test --test nonfunctional_regression_gates -- --ignored --nocapture
```

Included gates:

- Queue throughput regression (`<= 5%` allowed degradation from baseline)
- Retry-path p95 runtime regression (`<= 7%` allowed increase from baseline)
- DB busy/lock incidence under concurrent writes (`<= 0.5%` of operations)

## Baseline knobs

The suite uses env-driven baselines so teams can calibrate on their CI runner without code edits:

- `NF_BASELINE_QUEUE_THROUGHPUT_OPS_PER_SEC`
- `NF_BASELINE_RETRY_PATH_P95_MS`

If unset, conservative defaults are used:

- Queue throughput baseline: `200 ops/s`
- Retry-path p95 baseline: `50 ms`

## Phase gate expectation

- Run the non-functional suite at start/end of each high-risk extraction phase.
- Any threshold breach is stop-the-line and requires rollback/replan review before proceeding.
