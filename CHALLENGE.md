# How to challenge a number

Every figure in this repository is falsifiable, and we would rather be corrected in public
than be wrong in private.

## If you are an author of a measured system

If we measured your system badly — wrong build flags, a stale commit, a configuration you
do not recommend, a task expressed in a way you would not have written — **open an issue
with the configuration you would use**. We will re-run it and publish both numbers, with
credit and date. We will not quietly replace the old one; the correction and its reason
stay in the record.

**We commit to reproducing your own published reference number before reporting anything
about your system.** If we could not reproduce it, that discrepancy is published *above*
our result, not in a footnote.

## If you think the methodology is wrong

Open an issue. The methodology is in [README.md](README.md) and
[TASKS.md](TASKS.md), and it is frozen before measurement precisely so that it can be
attacked independently of the results.

Arguments we consider strong:

- The MAC count for a task is wrong. This changes every derived figure and we want to know
  immediately.
- A task's expression favours one protocol family in a way we did not declare.
- The correctness control is insufficient — i.e. a system could pass it while proving
  nothing.
- **A cross-system, per-operation memory benchmark already exists.** Then this repository
  should not exist in its current form, and we will say so and link to yours.

## What we will not do

- Remove an unflattering number, including our own.
- Publish a comparison where the security models differ without declaring it in the same
  table.
- Report a figure without its full conditions line.
- Extrapolate outside the measured range. We did that once, it cost us a strategic decision,
  and the rule now is absolute.
