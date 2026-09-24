use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::mode::Mode;

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuReading {
    pub temp: Option<f32>,
    pub usage: Option<f32>,
    pub clock_mhz: Option<f32>,
    pub power_w: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum GpuStatus {
    Active(GpuReading),
    Sleeping,
    #[default]
    Absent,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SensorReading {
    pub cpu_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub sys_temp: Option<f32>,
    pub ssd_temp: Option<f32>,
    pub cpu_fan_rpm: Option<u32>,
    pub gpu_fan_rpm: Option<u32>,
    pub cpu_usage: Option<f32>,
    pub ram_used_gb: Option<f32>,
    pub ram_total_gb: Option<f32>,
    pub gpu: GpuStatus,
}

pub struct Sensors {
    sys: PathBuf,
    proc_root: PathBuf,
    prev_cpu: Option<(u64, u64)>,
    gpu: Box<dyn FnMut() -> Option<GpuReading> + Send>,
}

fn read_num(p: &Path) -> Option<f64> {
    fs::read_to_string(p).ok()?.trim().parse().ok()
}

fn find_hwmon(sys: &Path, name: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(sys.join("class/hwmon")).ok()?.flatten().map(|e| e.path()).collect();
    dirs.sort();
    dirs.into_iter()
        .find(|p| fs::read_to_string(p.join("name")).map(|n| n.trim() == name).unwrap_or(false))
}

/// Temperatura em °C; 0 ou ausente vira None.
fn temp(dir: &Option<PathBuf>, file: &str) -> Option<f32> {
    let v = read_num(&dir.as_ref()?.join(file))?;
    (v > 0.0).then(|| (v / 1000.0) as f32)
}

fn rpm(dir: &Option<PathBuf>, file: &str) -> Option<u32> {
    read_num(&dir.as_ref()?.join(file)).map(|v| v as u32)
}

pub fn parse_nvidia_smi(out: &str) -> Option<GpuReading> {
    let line = out.lines().next()?;
    let f: Vec<Option<f32>> = line.split(',').map(|x| x.trim().parse().ok()).collect();
    if f.len() != 4 {
        return None;
    }
    Some(GpuReading { temp: f[0], usage: f[1], clock_mhz: f[2], power_w: f[3] })
}

fn nvidia_smi() -> Option<GpuReading> {
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=temperature.gpu,utilization.gpu,clocks.gr,power.draw",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_nvidia_smi(&String::from_utf8_lossy(&out.stdout))
}

impl Sensors {
    pub fn new(sys: PathBuf, proc_root: PathBuf, gpu: Box<dyn FnMut() -> Option<GpuReading> + Send>) -> Self {
        Sensors { sys, proc_root, prev_cpu: None, gpu }
    }

    pub fn system() -> Self {
        Self::new("/sys".into(), "/proc".into(), Box::new(nvidia_smi))
    }

    pub fn read_mode(&self) -> Option<Mode> {
        let s = fs::read_to_string(self.sys.join("firmware/acpi/platform_profile")).ok()?;
        Mode::from_token(&s)
    }

    /// Placa de vídeo NVIDIA (classe 0x03xxxx): Some(true) se ativa, Some(false) se dormindo.
    fn gpu_active(&self) -> Option<bool> {
        for e in fs::read_dir(self.sys.join("bus/pci/devices")).ok()?.flatten() {
            let p = e.path();
            let vendor = fs::read_to_string(p.join("vendor")).unwrap_or_default();
            let class = fs::read_to_string(p.join("class")).unwrap_or_default();
            if vendor.trim() == "0x10de" && class.trim().starts_with("0x03") {
                let st = fs::read_to_string(p.join("power/runtime_status")).unwrap_or_default();
                return Some(st.trim() == "active");
            }
        }
        None
    }

    fn cpu_usage(&mut self) -> Option<f32> {
        let stat = fs::read_to_string(self.proc_root.join("stat")).ok()?;
        let nums: Vec<u64> = stat.lines().next()?.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
        if nums.len() < 5 {
            return None;
        }
        let total: u64 = nums.iter().take(8).sum();
        let idle = nums[3] + nums[4];
        let prev = self.prev_cpu.replace((total, idle))?;
        let dt = total.saturating_sub(prev.0);
        if dt == 0 {
            return None;
        }
        let di = idle.saturating_sub(prev.1);
        Some(((1.0 - di as f64 / dt as f64) * 100.0) as f32)
    }

    fn memory(&self) -> (Option<f32>, Option<f32>) {
        let Ok(m) = fs::read_to_string(self.proc_root.join("meminfo")) else { return (None, None) };
        let field = |name: &str| -> Option<f64> {
            m.lines().find(|l| l.starts_with(name))?.split_whitespace().nth(1)?.parse().ok()
        };
        let gb = |kb: f64| (kb / 1024.0 / 1024.0) as f32;
        match (field("MemTotal:"), field("MemAvailable:")) {
            (Some(t), Some(a)) => (Some(gb(t - a)), Some(gb(t))),
            (Some(t), None) => (None, Some(gb(t))),
            _ => (None, None),
        }
    }

    pub fn read(&mut self) -> SensorReading {
        let acer = find_hwmon(&self.sys, "acer");
        let core = find_hwmon(&self.sys, "coretemp");
        let acpi = find_hwmon(&self.sys, "acpitz");
        let nvme = find_hwmon(&self.sys, "nvme");
        let gpu = match self.gpu_active() {
            Some(true) => (self.gpu)().map(GpuStatus::Active).unwrap_or(GpuStatus::Absent),
            Some(false) => GpuStatus::Sleeping,
            None => GpuStatus::Absent,
        };
        let gpu_temp = match &gpu {
            GpuStatus::Active(g) => g.temp.or_else(|| temp(&acer, "temp2_input")),
            _ => None,
        };
        let (ram_used_gb, ram_total_gb) = self.memory();
        SensorReading {
            cpu_temp: temp(&core, "temp1_input").or_else(|| temp(&acer, "temp1_input")),
            gpu_temp,
            sys_temp: temp(&acer, "temp3_input").or_else(|| temp(&acpi, "temp1_input")),
            ssd_temp: temp(&nvme, "temp1_input"),
            cpu_fan_rpm: rpm(&acer, "fan1_input"),
            gpu_fan_rpm: rpm(&acer, "fan2_input"),
            cpu_usage: self.cpu_usage(),
            ram_used_gb,
            ram_total_gb,
            gpu,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn w(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }

    /// Árvore falsa parecida com a máquina real.
    fn fake_tree() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let sys = d.path().join("sys");
        w(&sys, "class/hwmon/hwmon1/name", "acpitz\n");
        w(&sys, "class/hwmon/hwmon1/temp1_input", "45000\n");
        w(&sys, "class/hwmon/hwmon3/name", "nvme\n");
        w(&sys, "class/hwmon/hwmon3/temp1_input", "35850\n");
        w(&sys, "class/hwmon/hwmon6/name", "coretemp\n");
        w(&sys, "class/hwmon/hwmon6/temp1_input", "57000\n");
        w(&sys, "class/hwmon/hwmon7/name", "acer\n");
        w(&sys, "class/hwmon/hwmon7/fan1_input", "4007\n");
        w(&sys, "class/hwmon/hwmon7/fan2_input", "4045\n");
        w(&sys, "class/hwmon/hwmon7/temp1_input", "57000\n");
        w(&sys, "class/hwmon/hwmon7/temp2_input", "0\n");
        w(&sys, "class/hwmon/hwmon7/temp3_input", "51000\n");
        w(&sys, "firmware/acpi/platform_profile", "performance\n");
        w(&sys, "bus/pci/devices/0000:01:00.0/vendor", "0x10de\n");
        w(&sys, "bus/pci/devices/0000:01:00.0/class", "0x030000\n");
        w(&sys, "bus/pci/devices/0000:01:00.0/power/runtime_status", "active\n");
        w(&sys, "bus/pci/devices/0000:01:00.1/vendor", "0x10de\n");
        w(&sys, "bus/pci/devices/0000:01:00.1/class", "0x040300\n");
        w(&sys, "bus/pci/devices/0000:01:00.1/power/runtime_status", "suspended\n");
        let proc_root = d.path().join("proc");
        w(&proc_root, "stat", "cpu  100 0 100 800 0 0 0 0 0 0\ncpu0 1 1 1 1\n");
        w(&proc_root, "meminfo", "MemTotal:       16777216 kB\nMemFree: 1 kB\nMemAvailable:    8388608 kB\n");
        d
    }

    fn gpu_ok() -> Box<dyn FnMut() -> Option<GpuReading> + Send> {
        Box::new(|| parse_nvidia_smi("43, 12, 1740, 62.5\n"))
    }

    fn sensors(d: &tempfile::TempDir, gpu: Box<dyn FnMut() -> Option<GpuReading> + Send>) -> Sensors {
        Sensors::new(d.path().join("sys"), d.path().join("proc"), gpu)
    }

    #[test]
    fn reads_real_layout() {
        let d = fake_tree();
        let mut s = sensors(&d, gpu_ok());
        let r = s.read();
        assert_eq!(r.cpu_temp, Some(57.0));
        assert_eq!(r.sys_temp, Some(51.0), "acer temp3 tem prioridade sobre acpitz");
        assert_eq!(r.ssd_temp, Some(35.85));
        assert_eq!(r.cpu_fan_rpm, Some(4007));
        assert_eq!(r.gpu_fan_rpm, Some(4045));
        assert_eq!(r.gpu_temp, Some(43.0));
        assert_eq!(r.ram_total_gb, Some(16.0));
        assert_eq!(r.ram_used_gb, Some(8.0));
        assert_eq!(r.cpu_usage, None, "primeira leitura não tem delta");
        match r.gpu {
            GpuStatus::Active(g) => {
                assert_eq!(g.usage, Some(12.0));
                assert_eq!(g.clock_mhz, Some(1740.0));
                assert_eq!(g.power_w, Some(62.5));
            }
            other => panic!("esperava Active, veio {other:?}"),
        }
    }

    #[test]
    fn cpu_usage_from_delta() {
        let d = fake_tree();
        let mut s = sensors(&d, gpu_ok());
        s.read();
        // +100 ocupado, +100 ocioso → 50%
        w(&d.path().join("proc"), "stat", "cpu  150 0 150 900 0 0 0 0 0 0\n");
        assert_eq!(s.read().cpu_usage, Some(50.0));
    }

    #[test]
    fn reads_mode_from_platform_profile() {
        let d = fake_tree();
        let s = sensors(&d, gpu_ok());
        assert_eq!(s.read_mode(), Some(crate::mode::Mode::Turbo));
    }

    #[test]
    fn unknown_profile_token_is_none() {
        let d = fake_tree();
        let s = sensors(&d, gpu_ok());
        w(&d.path().join("sys"), "firmware/acpi/platform_profile", "custom\n");
        assert_eq!(s.read_mode(), None);
        w(&d.path().join("sys"), "firmware/acpi/platform_profile", "quiet\n");
        assert_eq!(s.read_mode(), Some(crate::mode::Mode::Quiet));
    }

    #[test]
    fn suspended_gpu_is_not_queried() {
        let d = fake_tree();
        w(&d.path().join("sys"), "bus/pci/devices/0000:01:00.0/power/runtime_status", "suspended\n");
        let calls = Arc::new(AtomicUsize::new(0));
        let c2 = calls.clone();
        let mut s = sensors(&d, Box::new(move || {
            c2.fetch_add(1, Ordering::SeqCst);
            None
        }));
        let r = s.read();
        assert_eq!(r.gpu, GpuStatus::Sleeping);
        assert_eq!(r.gpu_temp, None, "acer temp2 = 0 com GPU dormindo vira None");
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn no_nvidia_device_is_absent() {
        let d = fake_tree();
        fs::remove_dir_all(d.path().join("sys/bus")).unwrap();
        let mut s = sensors(&d, gpu_ok());
        assert_eq!(s.read().gpu, GpuStatus::Absent);
    }

    #[test]
    fn finds_hwmon_after_renumbering() {
        let d = fake_tree();
        let mut s = sensors(&d, gpu_ok());
        assert_eq!(s.read().cpu_fan_rpm, Some(4007));
        let h = d.path().join("sys/class/hwmon");
        fs::rename(h.join("hwmon7"), h.join("hwmon8")).unwrap();
        assert_eq!(s.read().cpu_fan_rpm, Some(4007));
    }

    #[test]
    fn missing_sensors_are_none() {
        let d = tempfile::tempdir().unwrap();
        let mut s = Sensors::new(d.path().join("sys"), d.path().join("proc"), Box::new(|| None));
        let r = s.read();
        assert_eq!(r, SensorReading::default());
        assert_eq!(s.read_mode(), None);
    }

    #[test]
    fn nvidia_smi_parsing() {
        let g = parse_nvidia_smi("43, 0, 210, 1.28\n").unwrap();
        assert_eq!(g.temp, Some(43.0));
        assert_eq!(g.usage, Some(0.0));
        let na = parse_nvidia_smi("43, [N/A], 210, [N/A]").unwrap();
        assert_eq!(na.usage, None);
        assert_eq!(na.power_w, None);
        assert!(parse_nvidia_smi("").is_none());
        assert!(parse_nvidia_smi("1, 2").is_none());
    }
}
