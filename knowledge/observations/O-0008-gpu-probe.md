---
id: O-0008
type: observation
status: preliminary
updated: 2026-11-09
added_by: agent
human_ratified: false
---
# Optional Burn batch probe

## Observed

The local NVIDIA RTX 3060 Laptop GPU ran a Burn 0.22.1 Vulkan density-channel
probe. Peak sampled whole-device VRAM delta was about 50 MiB, below the soft
1024 MiB target. The predictor realifies full complex superoperators and checks
final probabilities against the f64 CPU implementation. The maximum checked
absolute error was approximately 5.8e-8.

Large uncached batches benefited from GPU throughput, but a single request was
slower and a persistent CPU cache was faster on every measured batch. The
optional feature is not installed as the cognitive runtime's execution backend.
See the GPU probe documentation and artifact for measured boundaries.

## Inferred

A GPU path may become useful for many distinct model candidates or larger state
spaces. The present repeated-query domain does not justify default GPU execution.

## Hypothesis

A bounded cache or batched candidate fitter could outperform repeated validation
and matrix application; this probe does not establish end-to-end fitting gains.

## Unknown

General-backend parity, cold shader compilation costs on other drivers, hard VRAM
quotas, and speed under changing model revisions. Sampled whole-device deltas
include other processes and can miss transient peaks.

## Relations

- *extends*: [O-0006](O-0006-si-execution.md)
- *relates*: [D-0001](../decisions/D-0001-developmental-kernel.md)

## Revisit when

The dimensionality, number of distinct candidates, or measured bottleneck changes.

Executed local verification: CPU nonclassical-channel realification test,
feature-enabled accelerator Clippy with warnings denied, and actual release GPU
probe with parity and memory monitoring. The published receipt and SHA256SUMS
identify measured source/binary artifacts; the instrument was uncommitted during
measurement. No claim of a hard memory quota or default runtime acceleration.
