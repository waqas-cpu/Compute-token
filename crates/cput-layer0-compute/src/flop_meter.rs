//! Simulated flop-meter — polls synthetic GPU telemetry at sub-epoch intervals.
//!
//! Production deployments replace [`SimulatedFlopMeter`] with a hardware adapter
//! (NVML, DCGM, ROCm). The simulation path exercises the same
//! [`TelemetrySample`] → attestation wiring without a physical GPU.

use crate::TelemetrySample;
use cput_core::policy::EPOCH_SECONDS;
use cput_core::units::Gflops;
use serde::{Deserialize, Serialize};

/// Configuration for deterministic GPU telemetry simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedFlopMeterConfig {
    /// Seconds between telemetry polls within an epoch.
    pub sample_interval_secs: u64,
    /// Rated sustained throughput in GFLOP/s (matches node registration).
    pub rated_gflops_per_sec: u128,
    /// Mean utilisation in basis points (0–10_000).
    pub util_bps: u16,
    /// Baseline thermal reading in °C.
    pub thermal_c: u16,
    /// Baseline power draw in watts.
    pub power_w: u32,
    /// Peak VRAM utilisation in basis points.
    pub vram_util_bps: u16,
}

impl Default for SimulatedFlopMeterConfig {
    fn default() -> Self {
        Self {
            sample_interval_secs: 30,
            rated_gflops_per_sec: 50_000_000,
            util_bps: 8_500,
            thermal_c: 68,
            power_w: 350,
            vram_util_bps: 7_200,
        }
    }
}

/// Deterministic flop-meter that synthesises periodic telemetry samples.
#[derive(Debug, Clone)]
pub struct SimulatedFlopMeter {
    cfg: SimulatedFlopMeterConfig,
}

impl SimulatedFlopMeter {
    /// Create a simulator with the given hardware profile.
    #[must_use]
    pub fn new(cfg: SimulatedFlopMeterConfig) -> Self {
        Self { cfg }
    }

    /// Poll interval in seconds.
    #[must_use]
    pub fn sample_interval_secs(&self) -> u64 {
        self.cfg.sample_interval_secs.max(1)
    }

    /// Produce one telemetry sample at logical poll index `tick`.
    ///
    /// Jitter is deterministic from `tick` so runs are reproducible in CI.
    #[must_use]
    pub fn sample_at(&self, tick: u64) -> TelemetrySample {
        let util = u128::from(self.cfg.util_bps);
        // ±3% utilisation swing over the epoch.
        let jitter_bps = i32::try_from(tick % 7).unwrap_or(0) - 3;
        let delta = util.saturating_mul(u128::from(jitter_bps.unsigned_abs())) / 100;
        let effective_util = if jitter_bps >= 0 {
            util.saturating_add(delta)
        } else {
            util.saturating_sub(delta)
        }
        .clamp(1, 10_000);

        let flops_per_sec = self
            .cfg
            .rated_gflops_per_sec
            .saturating_mul(effective_util)
            .saturating_mul(1_000_000_000)
            / 10_000;

        let thermal_drift = u16::try_from(tick % 5).unwrap_or(0);
        let thermal_c = self.cfg.thermal_c.saturating_add(thermal_drift);
        let power_w = self
            .cfg
            .power_w
            .saturating_mul(u32::try_from(effective_util).unwrap_or(1))
            / 10_000;

        TelemetrySample {
            flops_per_sec,
            vram_util_bps: self.cfg.vram_util_bps,
            thermal_c,
            power_w,
        }
    }

    /// Collect all samples for one epoch window (default 360 s).
    #[must_use]
    pub fn collect_epoch_samples(&self) -> Vec<TelemetrySample> {
        self.collect_samples_for_duration(EPOCH_SECONDS)
    }

    /// Collect samples covering `duration_secs` at the configured interval.
    #[must_use]
    pub fn collect_samples_for_duration(&self, duration_secs: u64) -> Vec<TelemetrySample> {
        let interval = self.sample_interval_secs();
        let count = duration_secs.div_ceil(interval);
        (0..count).map(|tick| self.sample_at(tick)).collect()
    }

    /// Integrate sample throughput into total epoch GFLOPs (R0.2 input).
    #[must_use]
    pub fn aggregate_gflops(samples: &[TelemetrySample], sample_interval_secs: u64) -> Gflops {
        let interval = sample_interval_secs.max(1);
        let total_flops: u128 = samples
            .iter()
            .map(|s| s.flops_per_sec.saturating_mul(u128::from(interval)))
            .sum();
        Gflops(total_flops / 1_000_000_000)
    }

    /// Peak thermal envelope across samples (R0.4 input).
    #[must_use]
    pub fn peak_thermal_c(samples: &[TelemetrySample]) -> u16 {
        samples.iter().map(|s| s.thermal_c).max().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_samples_cover_full_window() {
        let meter = SimulatedFlopMeter::new(SimulatedFlopMeterConfig {
            sample_interval_secs: 30,
            ..Default::default()
        });
        let samples = meter.collect_epoch_samples();
        assert_eq!(samples.len(), 12);
        assert!(samples[0].flops_per_sec > 0);
    }

    #[test]
    fn aggregate_respects_capacity_fraction() {
        let meter = SimulatedFlopMeter::new(SimulatedFlopMeterConfig {
            sample_interval_secs: 360,
            rated_gflops_per_sec: 10,
            util_bps: 10_000,
            ..Default::default()
        });
        let samples = meter.collect_epoch_samples();
        let total = SimulatedFlopMeter::aggregate_gflops(&samples, 360);
        // 10 GFLOP/s × 360 s × 100% util ≈ 3600 GFLOPs (jitter may shave a few %).
        assert!(total.0 >= 3_400 && total.0 <= 3_700);
    }
}
