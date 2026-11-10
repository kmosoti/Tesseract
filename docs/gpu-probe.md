# Optional Burn GPU batch probe

The experimental accelerator realifies each complex Kraus superoperator into a
32-by-32 real matrix and evaluates batches with Burn 0.22.1 on Vulkan. Off-diagonal
coherences are retained; this is not a replacement with classical transition
probabilities. Default builds and the cognitive runtime remain CPU based.

```bash
cargo run --locked --release -p tesseract-accelerator --features wgpu -- runs/demo.jsonl
```

Use a real or complex density-model journal, not the Bayesian representation.
The explicit device selector is Vulkan discrete adapter 0. NVIDIA telemetry uses
index 0; their identity is an assumption supported by this host having a single
discrete RTX 3060 Laptop GPU (verified with PCI inventory), not a portable adapter
identity guarantee. The CPU was an Intel Core i7-10750H; Rust was 1.100.0-beta.7.

The fixed benchmark uses a nonporous three-action heat/press/cool program,
initial states cycling 0..3, and batches 1, 32, 256 and 4096. GPU time includes
state upload, compute and synchronized full-state readback; operator construction
and upload occur before warm timings. Each size has an untimed warm-up and ten
measured repetitions. Device/model initialization is reported separately; shader
caches may already exist. CPU f64 prediction validates the model per batch, while
GPU operators are cached. CPU cache baselines include both rebuilding its four
unique predictions and persistently reusing those predictions. These are different
preprocessing boundaries and are labeled in the receipt.

The observed VRAM increase was approximately 50 MiB (whole-device baseline about
181 MiB, peak about 231 MiB). Tensors are bounded to 4096 rows and the monitor exits
if sampled whole-device growth exceeds 1024 MiB. This is a soft guard, not a hard
driver allocation quota; 100 ms sampling can miss transients, and other processes
can influence the delta. Do not infer a hard cap from the allocator policy.

At this scale the GPU was slower for individual requests and faster for large
uncached batches. Persistently cached CPU queries were faster at every size.
The finite domain therefore does not justify default GPU inference. This probe
does not accelerate fitting or claim general backend qualification. Maximum
checked final-probability error was about 5.8e-8 against f64 CPU predictions;
nonfinite output or error above 1e-5 fails. CPU-only tests additionally use
nonclassical complex channels, but the GPU measurement itself covers only the
fixed program/context and final readouts, not every state or program.

The [receipt](../evidence/gpu-probe-v1/receipt.json) and
[source/binary hashes](../evidence/gpu-probe-v1/SHA256SUMS) identify the measured
artifact. Timings are observations for these conditions, not universal speedups.
