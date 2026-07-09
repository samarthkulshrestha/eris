# Kill-test: pre-registered criteria

Committed before the run. Not edited afterward. Results are reported against
this document unchanged.

**Question.** Does the structured surface (language card + cookbook + JSON
diagnostics + `--explain`) measurably improve an LLM's ability to write
correct Eris, compared to plain prose documentation (LANGUAGE.md) with
plain-text compiler errors?

**Design.** 12 held-out tasks (eval/tasks/tasks.json) × 2 conditions
(plain, structured) × 3 independent samples, model `claude-sonnet-5`,
max 6 repair turns per attempt. n = 36 attempts per condition.

**Primary metric.** Solve rate (task solved within 6 turns).
**Secondary metric.** Mean turns-to-solve among solved attempts.

**PROCEED to Phase C if:** structured − plain ≥ +10 percentage points
absolute on solve rate, OR mean turns-to-solve is reduced by ≥ 25% with
solve rate no worse.

**KILL (thesis as stated) otherwise:** write eval/results/NEGATIVE.md
summarizing the numbers, stop Phases C and D. The project's value is then a
portfolio artifact; no further eval investment.

**Budget.** ≤ ~$150 API spend. If the run must be aborted mid-way, partial
results are reported as partial — not extrapolated.
