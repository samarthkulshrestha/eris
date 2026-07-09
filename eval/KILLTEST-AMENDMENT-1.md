# Amendment 1 — transport change (pre-run)

Dated 2026-07-10, before any experimental data was collected.

The API account for the pre-registered run has no credits; the run executes
via Claude Code headless mode (`claude -p`, subscription-billed) instead of
the Anthropic SDK. Model (`claude-sonnet-5`), tasks, conditions, sample
count, turn cap, metrics, and decision thresholds are unchanged. The
transport wraps both experimental conditions identically (same system-prompt
replacement, same message flattening, no tools), so the between-condition
comparison the criteria are defined over is unaffected. Absolute solve rates
are not comparable to a hypothetical raw-API run.
