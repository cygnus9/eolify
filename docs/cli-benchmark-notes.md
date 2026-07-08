# eolify CLI Benchmark Notes

These are preliminary benchmark notes from running the `eolify` CLI against a
Linux kernel checkout. They are intended as a snapshot for discussion, not as a
formal benchmark suite.

## Environment

- Repository: Linux kernel checkout
- Tracked paths: `94,837` from `git ls-files | wc -l`
- Input source: `git ls-files -z`
- eolify command: `check --files-from - -0 --to lf -q`
- Filesystem state: likely warm page cache

The warm-cache detail matters. These results mostly measure path lookup,
metadata/open overhead, kernel memory reads, classification, and line-ending
analysis. They should not be interpreted as cold-disk throughput numbers.

## Baseline Commands

Path-list overhead:

```fish
time git ls-files -z >/dev/null
time git ls-files -z | xargs -0 -n 1000 true
```

Content-scan baselines:

```fish
time git grep -Il \$'\r' >/dev/null
time git grep -Il \$'\r' -- . >/dev/null
time git ls-files -z | xargs -0 dos2unix -ic0 >/dev/null
```

eolify worker scaling:

```fish
for j in 1 2 4 8 16 32
    echo "-j $j"
    time git ls-files -z | ~/git/eolify/target/release/eolify check --files-from - -0 --to lf -q -j $j
end
```

## Observed Results

### Baselines

| Command | Wall time | User time | Sys time |
|---|---:|---:|---:|
| `git ls-files -z >/dev/null` | 13.05 ms | 7.27 ms | 5.92 ms |
| `git ls-files -z \| xargs -0 -n 1000 true` | 54.18 ms | 48.01 ms | 25.30 ms |
| `git grep -Il $'\r' >/dev/null` | 423.83 ms | 1.83 s | 0.75 s |
| `git grep -Il $'\r' -- . >/dev/null` | 421.08 ms | 1.83 s | 0.77 s |
| `git ls-files -z \| xargs -0 dos2unix -ic0 >/dev/null` | 5.23 s | 4.78 s | 0.46 s |

`dos2unix` also reported symbolic links whose targets are not regular files in
the Linux tree. This matches the behavior expected from eolify's default policy:
non-regular filesystem entries should be skipped by default rather than treated
as check failures.

### eolify Worker Scaling

| Jobs | Wall time | User time | Sys time |
|---:|---:|---:|---:|
| 1 | 1.09 s | 673.13 ms | 532.33 ms |
| 2 | 595.37 ms | 693.95 ms | 601.91 ms |
| 4 | 375.25 ms | 814.41 ms | 759.95 ms |
| 8 | 282.26 ms | 1.06 s | 0.96 s |
| 16 | 280.81 ms | 1.14 s | 0.87 s |
| 32 | 284.11 ms | 1.12 s | 0.89 s |

An uncapped default run using available parallelism was also observed around
`277 ms` wall time on this machine.

## Preliminary Interpretation

For this repository and machine, eolify's quiet check mode is fast enough that
it should not be intrusive in typical developer workflows:

- It is much slower than path enumeration alone, as expected, because it opens,
  classifies, and analyzes file contents.
- It was faster than `git grep -Il $'\r'` in wall time and used less total CPU
  in this run.
- It was far faster than `dos2unix -ic0` for this file-list workflow.

The worker-scaling result suggests this workload is not dominated by waiting on
physical disk I/O in the warm-cache case. Parallelism helps up to the point where
available CPU and syscall capacity are saturated, then plateaus. On this machine
there was no meaningful wall-time improvement beyond `-j 8`.

That does not prove that `8` is a universal cap. It only shows that more workers
did not help on this machine and this workload. The current default of using
available parallelism remains reasonable. A hard cap would be a policy choice
for large shared machines, not a conclusion proven by these measurements.

## Caveats

- These are single-machine, informal measurements.
- The filesystem cache was likely warm.
- Fish shell's `time` output separates shell and external process timing; the
  table uses the total values shown by `time`.
- `git grep`, `dos2unix`, and eolify do not perform identical work. They are
  useful workflow baselines, not exact semantic equivalents.
- Output was quiet for eolify. Non-quiet output can dominate runtime when many
  files are reported.

## Follow-up Ideas

- Repeat on a cold checkout or CI machine.
- Compare debug and release builds only when validating development overhead.
- Measure with JSON output and non-quiet output separately.
- Track benchmark results over time with a small reproducible fixture plus at
  least one large real repository.
