# Kill-test result: KILL (thesis as stated)

Run completed 2026-07-10. Design and thresholds per the pre-registered
`eval/KILLTEST.md` (committed before the runner existed) with the transport
amendment in `eval/KILLTEST-AMENDMENT-1.md` (claude-code headless backend,
recorded before any data was collected).

## Numbers

| Condition  | Solve rate | Mean turns-to-solve |
|------------|-----------:|--------------------:|
| plain      | 36/36 (100%) | 1.00 |
| structured | 36/36 (100%) | 1.00 |

12 tasks × 2 conditions × 3 samples = 72 attempts, model `claude-sonnet-5`,
max 6 repair turns. Raw data: `eval/results/killtest.csv`.

## Verdict against pre-registered criteria

- structured − plain solve rate: **0 points** (threshold: ≥ +10) — not met.
- turns-to-solve reduction: **0%** (threshold: ≥ 25%) — not met.

**KILL.** Per the pre-registration, Phases C and D do not proceed.

## What the data actually shows

This is a **ceiling-effect null**, and a total one. `claude-sonnet-5` wrote a
correct Eris program on the first attempt for every task in both conditions.
Zero compile errors occurred in 72 attempts, which means:

1. **Plain prose documentation alone (LANGUAGE.md, ~700 lines) is fully
   sufficient** for a 2026 frontier model to one-shot small programs in a
   language absent from its training data. The gap the structured surface
   was built to close does not exist at this task difficulty and model
   strength.
2. **The repair loop — and therefore the structured error machinery
   (JSON diagnostics, stable codes, `--explain`) — was never exercised.**
   The experiment could not have differentiated the error-explanation
   surface even in principle, because no error ever reached the model.
3. The pre-mortem prediction ("the baseline improving is the thing that
   falsifies the paper") was correct, and understated: the baseline was
   already saturated before the project began measuring.

## What would constitute a new, different experiment

Not authorized by this pre-registration; each would require a fresh
pre-registered design:

- Substantially harder tasks (multi-function programs, edge-case-dense
  specs) to pull both conditions off the ceiling.
- Weaker/smaller models, where a docs-vs-structured gap may still exist.
- A no-docs floor condition to quantify how much *any* documentation
  contributes vs. the model guessing from examples alone.

## Cost

Subscription-billed (Claude Max, claude-code backend); 72 model calls,
one turn each. No API credits spent.
