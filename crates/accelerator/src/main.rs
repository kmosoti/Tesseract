use burn::tensor::{
    Device, DeviceKind, Tensor, TensorData,
    wgpu::{MemoryConfiguration, WgpuBackend},
};
use std::{
    error::Error,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tesseract_accelerator::{MAX_BATCH, cpu_program, operators};
use tesseract_contracts::{Action, Features};
use tesseract_journal::Session;
fn memory() -> Result<u64, Box<dyn Error>> {
    let output = Command::new("nvidia-smi")
        .args([
            "--id=0",
            "--query-gpu=memory.used",
            "--format=csv,noheader,nounits",
        ])
        .output()?;
    if !output.status.success() {
        return Err("VRAM telemetry unavailable".into());
    }
    Ok(std::str::from_utf8(&output.stdout)?.trim().parse()?)
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 1 {
        return Err("usage: tesseract-gpu-probe JOURNAL".into());
    }
    let session = Session::open(args.first().ok_or("usage: tesseract-gpu-probe JOURNAL")?)?;
    let model = session.kernel().model();
    let baseline = memory()?;
    let stop = Arc::new(AtomicBool::new(false));
    let peak = Arc::new(AtomicU64::new(baseline));
    let stopped = stop.clone();
    let peak_observed = peak.clone();
    // Sampled whole-GPU delta includes other applications; this is not a hard
    // device allocation quota. Fail conservatively if the requested budget is crossed.
    let monitor = std::thread::spawn(move || {
        while !stopped.load(Ordering::Relaxed) {
            match memory() {
                Ok(used) => {
                    peak_observed.fetch_max(used, Ordering::Relaxed);
                    if used.saturating_sub(baseline) > 1024 {
                        eprintln!("VRAM delta exceeded 1024 MiB");
                        std::process::exit(2);
                    }
                }
                Err(error) => {
                    eprintln!("VRAM telemetry failed: {error}");
                    std::process::exit(2);
                }
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    let cold = Instant::now();
    let device = Device::wgpu_options()
        .device_kind(DeviceKind::DiscreteGpu(0))
        .graphics_api(WgpuBackend::Vulkan)
        .memory_config(MemoryConfiguration::ExclusivePages)
        .tasks_max(8)
        .init()?;
    let compiled: Vec<Tensor<2>> = operators(model)?
        .into_iter()
        .map(|data| Tensor::from_data(TensorData::new(data, [32, 32]), &device))
        .collect();
    device.sync()?;
    let cold_ms = cold.elapsed().as_secs_f64() * 1000.0;
    let mut reports = Vec::new();
    let actions = [Action::Heat, Action::Press, Action::Cool];
    for batch in [1, 32, 256, MAX_BATCH] {
        let initials: Vec<_> = (0..batch).map(|i| (i % 4) as u8).collect();
        let input: Vec<f32> = (0..batch * 32)
            .map(|i| f32::from(i % 32 == (initials[i / 32] as usize) * 5))
            .collect();
        let gpu = || -> Result<Vec<f32>, Box<dyn Error>> {
            let mut state =
                Tensor::<2>::from_data(TensorData::new(input.clone(), [batch, 32]), &device);
            for action in actions {
                state = state.matmul(compiled[action.index()].clone());
            }
            Ok(state.try_into_data()?.try_to_vec::<f32>()?)
        };
        // Warm shader compilation/autotuning separately from synchronized timings.
        let _ = gpu()?;
        let expected = cpu_program(model, Features { porous: false }, &actions, &initials)?;
        let found = gpu()?;
        if found.iter().any(|x| !x.is_finite()) {
            return Err("nonfinite GPU output".into());
        }
        let mut error = 0f64;
        for (i, probabilities) in expected.iter().enumerate() {
            for outcome in 0..4 {
                error =
                    error.max((found[i * 32 + outcome * 5] as f64 - probabilities[outcome]).abs());
            }
        }
        if error > 1e-5 {
            return Err(format!("GPU parity failure: {error}").into());
        }
        let started = Instant::now();
        for _ in 0..10 {
            std::hint::black_box(gpu()?);
        }
        let gpu_ms = started.elapsed().as_secs_f64() * 1000.0 / 10.0;
        let started = Instant::now();
        for _ in 0..10 {
            std::hint::black_box(cpu_program(
                model,
                Features { porous: false },
                &actions,
                &initials,
            )?);
        }
        let cpu_ms = started.elapsed().as_secs_f64() * 1000.0 / 10.0;
        // Competent repeated-query baseline: the bounded domain has only four
        // distinct inputs for this program. Include rebuilding the cache per batch.
        let started = Instant::now();
        for _ in 0..10 {
            let unique = cpu_program(model, Features { porous: false }, &actions, &[0, 1, 2, 3])?;
            std::hint::black_box(
                initials
                    .iter()
                    .map(|i| unique[*i as usize])
                    .collect::<Vec<_>>(),
            );
        }
        let cached_cpu_ms = started.elapsed().as_secs_f64() * 1000.0 / 10.0;
        let unique = cpu_program(model, Features { porous: false }, &actions, &[0, 1, 2, 3])?;
        let started = Instant::now();
        for _ in 0..10 {
            std::hint::black_box(
                initials
                    .iter()
                    .map(|i| unique[*i as usize])
                    .collect::<Vec<_>>(),
            );
        }
        let persistent_cpu_cache_ms = started.elapsed().as_secs_f64() * 1000.0 / 10.0;
        reports.push(serde_json::json!({"batch":batch,"gpu_ms_upload_compute_readback":gpu_ms,"cpu_ms_uncached":cpu_ms,"cpu_ms_cache_rebuilt":cached_cpu_ms,"cpu_ms_cache_persistent":persistent_cpu_cache_ms,"max_abs_error":error}));
        device.memory_cleanup();
    }
    stop.store(true, Ordering::Relaxed);
    monitor.join().map_err(|_| "monitor panicked")?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"wgpu_selector":"Vulkan DiscreteGpu(0)","nvidia_telemetry_index":0,"adapter_identity_assumption":"single discrete GPU verified locally by lspci", "source_model_hash":tesseract_contracts::digest(model)?,"source_revision_id":session.kernel().revision(),"journal_head":session.head(),"burn":"0.22.1","cold_device_and_model_ms":cold_ms,"gpu_baseline_mib":baseline,"gpu_peak_observed_mib":peak.load(Ordering::Relaxed),"gpu_delta_peak_observed_mib":peak.load(Ordering::Relaxed).saturating_sub(baseline),"soft_vram_budget_mib":1024,"repetitions":10,"results":reports,"scope":"fixed 3-action program; f32 realified channel vs f64 authoritative CPU; no runtime backend replacement"})
        )?
    );
    Ok(())
}
