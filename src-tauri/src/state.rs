use std::collections::VecDeque;

use serde::Serialize;

use crate::asense::protocol::PlatformState;
use crate::mode::{FanMode, Mode};
use crate::sensors::{GpuStatus, SensorReading};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Connection {
    Connected,
    #[default]
    Disconnected,
    Busy,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub connection: Connection,
    pub mode: Option<Mode>,
    pub fan_mode: Option<FanMode>,
    pub sensors: SensorReading,
    pub platform: Option<PlatformState>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Snapshot(Snapshot),
    ModeChanged { from: Option<Mode>, to: Mode },
    Toast(String),
}

pub fn diff(prev: &Snapshot, next: &Snapshot) -> Vec<Event> {
    let mut ev = Vec::new();
    if let Some(to) = next.mode {
        if prev.mode != Some(to) {
            ev.push(Event::ModeChanged { from: prev.mode, to });
        }
    }
    if prev != next {
        ev.push(Event::Snapshot(next.clone()));
    }
    ev
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sample {
    pub t: u64,
    pub cpu_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub cpu_usage: Option<f32>,
    pub gpu_usage: Option<f32>,
    pub gpu_power: Option<f32>,
    pub cpu_fan: Option<f32>,
    pub gpu_fan: Option<f32>,
}

impl Sample {
    pub fn from_reading(r: &SensorReading, t: u64) -> Sample {
        let (gpu_usage, gpu_power) = match &r.gpu {
            GpuStatus::Active(g) => (g.usage, g.power_w),
            _ => (None, None),
        };
        Sample {
            t,
            cpu_temp: r.cpu_temp,
            gpu_temp: r.gpu_temp,
            cpu_usage: r.cpu_usage,
            gpu_usage,
            gpu_power,
            cpu_fan: r.cpu_fan_rpm.map(|v| v as f32),
            gpu_fan: r.gpu_fan_rpm.map(|v| v as f32),
        }
    }
}

pub struct History {
    cap: usize,
    buf: VecDeque<Sample>,
}

impl History {
    pub fn new(cap: usize) -> Self {
        History { cap, buf: VecDeque::with_capacity(cap) }
    }
    pub fn push(&mut self, s: Sample) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(s);
    }
    pub fn samples(&self) -> Vec<Sample> {
        self.buf.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensors::GpuReading;

    fn snap(mode: Option<Mode>) -> Snapshot {
        Snapshot { connection: Connection::Connected, mode, ..Default::default() }
    }

    #[test]
    fn unchanged_snapshot_emits_nothing() {
        let a = snap(Some(Mode::Eco));
        assert!(diff(&a, &a.clone()).is_empty());
    }

    #[test]
    fn mode_change_emits_mode_changed_then_snapshot() {
        let a = snap(Some(Mode::Balanced));
        let b = snap(Some(Mode::Turbo));
        let ev = diff(&a, &b);
        assert_eq!(ev[0], Event::ModeChanged { from: Some(Mode::Balanced), to: Mode::Turbo });
        assert!(matches!(ev[1], Event::Snapshot(_)));
    }

    #[test]
    fn first_known_mode_is_a_change() {
        let ev = diff(&snap(None), &snap(Some(Mode::Quiet)));
        assert_eq!(ev[0], Event::ModeChanged { from: None, to: Mode::Quiet });
    }

    #[test]
    fn losing_mode_is_not_a_mode_change() {
        let ev = diff(&snap(Some(Mode::Quiet)), &snap(None));
        assert_eq!(ev.len(), 1);
        assert!(matches!(ev[0], Event::Snapshot(_)));
    }

    #[test]
    fn sensor_change_emits_snapshot_only() {
        let a = snap(Some(Mode::Eco));
        let mut b = a.clone();
        b.sensors.cpu_temp = Some(70.0);
        let ev = diff(&a, &b);
        assert_eq!(ev.len(), 1);
    }

    #[test]
    fn history_is_bounded() {
        let mut h = History::new(3);
        for t in 0..5 {
            h.push(Sample { t, ..Default::default() });
        }
        let s = h.samples();
        assert_eq!(s.iter().map(|x| x.t).collect::<Vec<_>>(), vec![2, 3, 4]);
    }

    #[test]
    fn sample_from_reading() {
        let r = SensorReading {
            cpu_temp: Some(60.0),
            cpu_fan_rpm: Some(4000),
            gpu: GpuStatus::Active(GpuReading { usage: Some(30.0), power_w: Some(50.0), ..Default::default() }),
            ..Default::default()
        };
        let s = Sample::from_reading(&r, 9);
        assert_eq!(s.t, 9);
        assert_eq!(s.cpu_fan, Some(4000.0));
        assert_eq!(s.gpu_usage, Some(30.0));
        assert_eq!(s.gpu_power, Some(50.0));
    }

    #[test]
    fn snapshot_serializes_for_js() {
        let mut s = snap(Some(Mode::Turbo));
        s.sensors.gpu = GpuStatus::Sleeping;
        let j = serde_json::to_value(&s).unwrap();
        assert_eq!(j["mode"], "turbo");
        assert_eq!(j["connection"], "connected");
        assert_eq!(j["sensors"]["gpu"]["state"], "sleeping");
        assert!(j["sensors"].get("cpuFanRpm").is_some());
    }
}
