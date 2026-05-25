use std::fs;
use std::io::{self, Write};
use std::process::Command;

use crate::model::{clamp_percent, format_percent, format_size_mb, Meter, Snapshot};

const TEMP_PATH: &str = "/sys/class/thermal/thermal_zone0/temp";
const TEMP_MIN_C: f64 = 20.0;
const TEMP_MAX_C: f64 = 100.0;
const BW_PATH: &str = "/proc/ax_proc/bw/bw";
const PERF_MONITOR_KO: &str = "/soc/ko/ax_perf_monitor.ko";
const BW_MAX_GB: f64 = 20.0;
const MEMINFO_PATH: &str = "/proc/meminfo";
const CMM_INFO_PATH: &str = "/proc/ax_proc/mem_cmm_info";
const NPU_ENABLE_PATH: &str = "/proc/ax_proc/npu/enable";
const NPU_TOP_PATH: &str = "/proc/ax_proc/npu/top";

#[derive(Default)]
pub struct CpuSampler {
    last_total: Option<u64>,
    last_idle: Option<u64>,
}

impl CpuSampler {
    fn sample(&mut self) -> io::Result<Option<f64>> {
        let text = fs::read_to_string("/proc/stat")?;
        let line = text
            .lines()
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "empty /proc/stat"))?;
        let fields = line
            .split_whitespace()
            .skip(1)
            .map(|value| value.parse::<u64>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        if fields.len() < 5 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected /proc/stat format",
            ));
        }

        let idle = fields[3] + fields[4];
        let total = fields.iter().sum::<u64>();

        let (Some(last_total), Some(last_idle)) = (self.last_total, self.last_idle) else {
            self.last_total = Some(total);
            self.last_idle = Some(idle);
            return Ok(None);
        };

        self.last_total = Some(total);
        self.last_idle = Some(idle);

        let total_delta = total.saturating_sub(last_total);
        let idle_delta = idle.saturating_sub(last_idle);
        if total_delta == 0 {
            return Ok(None);
        }

        Ok(Some(clamp_percent(
            (1.0 - idle_delta as f64 / total_delta as f64) * 100.0,
        )))
    }
}

#[derive(Default)]
pub struct MetricReader {
    cpu: CpuSampler,
    npu_enabled: bool,
    perf_checked: bool,
    perf_error: Option<String>,
}

impl MetricReader {
    pub fn snapshot(&mut self) -> Snapshot {
        let meters = vec![
            self.safe_read("CPU", Self::read_cpu),
            self.safe_read("DDR OS", Self::read_os_memory),
            self.safe_read("DDR CMM", Self::read_cmm_memory),
            self.safe_read("NPU", Self::read_npu),
            self.safe_read("SoC Temp", Self::read_temp),
            self.safe_read("All BW", Self::read_bw),
        ];
        Snapshot { meters }
    }

    fn safe_read(
        &mut self,
        title: &'static str,
        reader: fn(&mut Self) -> Result<Meter, String>,
    ) -> Meter {
        match reader(self) {
            Ok(meter) => meter,
            Err(err) => Meter::unavailable(title, err),
        }
    }

    fn read_cpu(&mut self) -> Result<Meter, String> {
        let percent = self.cpu.sample().map_err(|err| err.to_string())?;
        let detail = percent
            .map(|value| format!("Current load {value:.1}%"))
            .unwrap_or_else(|| "Sampling...".to_string());
        Ok(Meter {
            title: "CPU",
            percent,
            value: format_percent(percent),
            detail,
        })
    }

    fn read_os_memory(&mut self) -> Result<Meter, String> {
        let text = fs::read_to_string(MEMINFO_PATH).map_err(|err| err.to_string())?;
        let mut total_kb = None;
        let mut available_kb = None;
        let mut free_kb = None;

        for line in text.lines() {
            let Some((key, rest)) = line.split_once(':') else {
                continue;
            };
            let value = rest
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<u64>().ok());
            match key {
                "MemTotal" => total_kb = value,
                "MemAvailable" => available_kb = value,
                "MemFree" => free_kb = value,
                _ => {}
            }
        }

        let total_kb = total_kb.ok_or("MemTotal unavailable")?;
        let available_kb = available_kb.or(free_kb).unwrap_or(0);
        let used_kb = total_kb.saturating_sub(available_kb);
        let percent = clamp_percent(used_kb as f64 / total_kb as f64 * 100.0);
        let detail = format!(
            "{} / {}",
            format_size_mb(used_kb as f64 / 1024.0),
            format_size_mb(total_kb as f64 / 1024.0)
        );

        Ok(Meter {
            title: "DDR OS",
            percent: Some(percent),
            value: format_percent(Some(percent)),
            detail,
        })
    }

    fn read_cmm_memory(&mut self) -> Result<Meter, String> {
        let text = fs::read_to_string(CMM_INFO_PATH).map_err(|err| err.to_string())?;
        let total_kb = parse_after(&text, "total size=", "KB")
            .ok_or_else(|| "CMM total unavailable".to_string())?;
        let used_kb = parse_after(&text, "used=", "KB")
            .ok_or_else(|| "CMM used unavailable".to_string())?;

        if total_kb == 0 {
            return Ok(Meter::unavailable("DDR CMM", "CMM total is zero"));
        }

        let percent = clamp_percent(used_kb as f64 / total_kb as f64 * 100.0);
        let detail = format!(
            "{} / {}",
            format_size_mb(used_kb as f64 / 1024.0),
            format_size_mb(total_kb as f64 / 1024.0)
        );

        Ok(Meter {
            title: "DDR CMM",
            percent: Some(percent),
            value: format_percent(Some(percent)),
            detail,
        })
    }

    fn read_npu(&mut self) -> Result<Meter, String> {
        let enable_error = self.ensure_npu_enabled().err();
        let output = fs::read_to_string(NPU_TOP_PATH)
            .map_err(|err| err.to_string())?
            .trim()
            .to_string();
        let lowered = output.to_ascii_lowercase();

        if output.is_empty() || lowered.contains("empty") || lowered.contains("not updated") {
            let detail = match enable_error {
                Some(err) => format!("No active workload | enable: {err}"),
                None => "No active workload".to_string(),
            };
            return Ok(Meter {
                title: "NPU",
                percent: Some(0.0),
                value: "0.0%".to_string(),
                detail,
            });
        }

        let percent = parse_after(&output, "utilization:", "%")
            .map(|value| clamp_percent(value as f64))
            .ok_or_else(|| "Unexpected npu/top format".to_string())?;
        let detail = match enable_error {
            Some(err) => format!("Current load {percent:.0}% | enable: {err}"),
            None => format!("Current load {percent:.0}%"),
        };

        Ok(Meter {
            title: "NPU",
            percent: Some(percent),
            value: format_percent(Some(percent)),
            detail,
        })
    }

    fn read_temp(&mut self) -> Result<Meter, String> {
        let raw = fs::read_to_string(TEMP_PATH).map_err(|err| err.to_string())?;
        let temp_c = raw
            .trim()
            .parse::<f64>()
            .map_err(|err| err.to_string())?
            / 1000.0;
        let percent = clamp_percent((temp_c - TEMP_MIN_C) / (TEMP_MAX_C - TEMP_MIN_C) * 100.0);

        Ok(Meter {
            title: "SoC Temp",
            percent: Some(percent),
            value: format!("{temp_c:.1} C"),
            detail: format!("{temp_c:.3} C"),
        })
    }

    fn read_bw(&mut self) -> Result<Meter, String> {
        let insmod_error = self.ensure_perf_monitor_loaded();
        let text = fs::read_to_string(BW_PATH).map_err(|err| err.to_string())?;
        let line = text
            .lines()
            .find(|line| line.starts_with("All BW:"))
            .map(str::trim)
            .ok_or_else(|| "BW info unavailable".to_string())?;
        let (value, unit) = parse_bw_line(line).ok_or_else(|| "Unexpected bw format".to_string())?;
        let value_gb = match unit {
            "KB" => value / (1024.0 * 1024.0),
            "MB" => value / 1024.0,
            "GB" => value,
            _ => return Err("Unexpected bw unit".to_string()),
        };
        let percent = clamp_percent(value_gb / BW_MAX_GB * 100.0);
        let value_text = if unit == "GB" {
            format!("{value:.2} GB")
        } else {
            format!("{value:.0} {unit}")
        };
        let detail = match insmod_error {
            Some(err) => format!("{line} | insmod: {err}"),
            None => line.to_string(),
        };

        Ok(Meter {
            title: "All BW",
            percent: Some(percent),
            value: value_text,
            detail,
        })
    }

    fn ensure_npu_enabled(&mut self) -> Result<(), String> {
        if self.npu_enabled {
            return Ok(());
        }

        let mut file = fs::OpenOptions::new()
            .write(true)
            .open(NPU_ENABLE_PATH)
            .map_err(|err| err.to_string())?;
        file.write_all(b"1\n").map_err(|err| err.to_string())?;
        self.npu_enabled = true;
        Ok(())
    }

    fn ensure_perf_monitor_loaded(&mut self) -> Option<String> {
        if self.perf_checked {
            return self.perf_error.clone();
        }
        self.perf_checked = true;

        if fs::read_to_string("/proc/modules")
            .map(|text| text.lines().any(|line| line.starts_with("ax_perf_monitor ")))
            .unwrap_or(false)
        {
            return None;
        }

        let output = Command::new("insmod").arg(PERF_MONITOR_KO).output();
        match output {
            Ok(output) if output.status.success() => None,
            Ok(output) => {
                let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if err.contains("File exists") {
                    None
                } else {
                    let err = if err.is_empty() {
                        "ax_perf_monitor.ko insmod failed".to_string()
                    } else {
                        err
                    };
                    self.perf_error = Some(err.clone());
                    Some(err)
                }
            }
            Err(err) => {
                let err = err.to_string();
                self.perf_error = Some(err.clone());
                Some(err)
            }
        }
    }
}

fn parse_after(text: &str, prefix: &str, suffix: &str) -> Option<u64> {
    let start = text.find(prefix)? + prefix.len();
    let rest = &text[start..];
    let end = rest.find(suffix)?;
    rest[..end].trim().parse().ok()
}

fn parse_bw_line(line: &str) -> Option<(f64, &'static str)> {
    let rest = line.strip_prefix("All BW:")?.trim();
    let mut parts = rest.split_whitespace();
    let value = parts.next()?.parse::<f64>().ok()?;
    let unit = match parts.next()? {
        "KB" => "KB",
        "MB" => "MB",
        "GB" => "GB",
        _ => return None,
    };
    Some((value, unit))
}

