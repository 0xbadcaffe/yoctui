//! Host telemetry.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CpuCounters {
    pub(crate) total: u64,
    pub(crate) idle: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DiskCounters {
    pub(crate) major: u64,
    pub(crate) minor: u64,
    pub(crate) read_bytes: u64,
    pub(crate) write_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NetworkCounters {
    pub(crate) interface: String,
    pub(crate) receive_bytes: u64,
    pub(crate) transmit_bytes: u64,
}

#[derive(Debug)]
pub(crate) struct HostTelemetrySampler {
    pub(crate) previous_cpu: Option<CpuCounters>,
    pub(crate) previous_disk: Option<DiskCounters>,
    pub(crate) previous_network: Option<NetworkCounters>,
    pub(crate) previous_sampled_at: Option<Instant>,
    pub(crate) logical_cpu_count: Option<u16>,
    pub(crate) build_disk_device: Option<(u64, u64)>,
    pub(crate) network_interface: Option<String>,
}

impl Default for HostTelemetrySampler {
    fn default() -> Self {
        Self {
            previous_cpu: None,
            previous_disk: None,
            previous_network: None,
            previous_sampled_at: None,
            logical_cpu_count: std::thread::available_parallelism()
                .ok()
                .and_then(|count| u16::try_from(count.get()).ok()),
            build_disk_device: None,
            network_interface: None,
        }
    }
}

impl HostTelemetrySampler {
    pub(crate) fn sample(&mut self, build_dir: &Path) -> HostTelemetry {
        self.sample_at(build_dir, Instant::now())
    }

    pub(crate) fn sample_at(&mut self, build_dir: &Path, sampled_at: Instant) -> HostTelemetry {
        let elapsed = self
            .previous_sampled_at
            .replace(sampled_at)
            .and_then(|previous| sampled_at.checked_duration_since(previous));
        let current_cpu = read_cpu_counters();
        let cpu_utilization_percent = current_cpu.and_then(|current| {
            let previous = self.previous_cpu.replace(current)?;
            let total = current.total.checked_sub(previous.total)?;
            let idle = current.idle.checked_sub(previous.idle)?;
            if total == 0 || idle > total {
                return None;
            }
            let percent = (total - idle).checked_mul(100)?.checked_div(total)?;
            Some(percent.min(100).try_into().unwrap_or(100))
        });
        let (memory_total_bytes, memory_available_bytes) = read_memory_info()
            .map_or((None, None), |(total, available)| {
                (Some(total), Some(available))
            });
        let (disk_available_bytes, disk_total_bytes) = disk_capacity_bytes(build_dir)
            .map_or((None, None), |(available, total)| {
                (Some(available), Some(total))
            });
        if self.build_disk_device.is_none() {
            self.build_disk_device = build_disk_device(build_dir);
        }
        let current_disk = self
            .build_disk_device
            .and_then(|(major, minor)| read_disk_counters(major, minor));
        if current_disk.is_none() {
            self.build_disk_device = None;
        }
        let (disk_read_bytes_per_second, disk_write_bytes_per_second) = elapsed
            .and_then(|elapsed| {
                disk_rates(
                    self.previous_disk.as_ref()?,
                    current_disk.as_ref()?,
                    elapsed,
                )
            })
            .map_or((None, None), |(read, write)| (Some(read), Some(write)));
        self.previous_disk = current_disk;
        if self.network_interface.is_none() {
            self.network_interface = read_default_network_interface();
        }
        let current_network = self
            .network_interface
            .as_deref()
            .and_then(read_network_counters);
        if current_network.is_none() {
            self.network_interface = None;
        }
        let (network_receive_bytes_per_second, network_transmit_bytes_per_second) = elapsed
            .and_then(|elapsed| {
                network_rates(
                    self.previous_network.as_ref()?,
                    current_network.as_ref()?,
                    elapsed,
                )
            })
            .map_or((None, None), |(receive, transmit)| {
                (Some(receive), Some(transmit))
            });
        self.previous_network = current_network;
        HostTelemetry {
            cpu_utilization_percent,
            logical_cpu_count: self.logical_cpu_count,
            memory_total_bytes,
            memory_available_bytes,
            disk_available_bytes,
            disk_total_bytes,
            disk_read_bytes_per_second,
            disk_write_bytes_per_second,
            network_receive_bytes_per_second,
            network_transmit_bytes_per_second,
            load_average_milli: read_load_average(),
        }
    }
}

pub(crate) fn read_cpu_counters() -> Option<CpuCounters> {
    let line = fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .next()?
        .to_owned();
    parse_cpu_counters(&line)
}

pub(crate) fn parse_cpu_counters(line: &str) -> Option<CpuCounters> {
    let mut fields = line.split_whitespace();
    (fields.next()? == "cpu").then_some(())?;
    let values = fields
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let total = values.iter().copied().sum();
    let idle = values.get(3).copied()? + values.get(4).copied().unwrap_or_default();
    Some(CpuCounters { total, idle })
}

pub(crate) fn read_memory_info() -> Option<(u64, u64)> {
    parse_memory_info(&fs::read_to_string("/proc/meminfo").ok()?)
}

pub(crate) fn parse_memory_info(input: &str) -> Option<(u64, u64)> {
    let mut total_kib = None;
    let mut available_kib = None;
    for line in input.lines() {
        let mut fields = line.split_whitespace();
        let Some(key) = fields.next() else {
            continue;
        };
        match key {
            "MemTotal:" => total_kib = parse_memory_kib(fields.next()?, fields.next()?),
            "MemAvailable:" => available_kib = parse_memory_kib(fields.next()?, fields.next()?),
            _ => continue,
        }
    }
    let total = total_kib?.checked_mul(1024)?;
    let available = available_kib?.checked_mul(1024)?;
    (total > 0 && available <= total).then_some((total, available))
}

pub(crate) fn parse_memory_kib(value: &str, unit: &str) -> Option<u64> {
    (unit == "kB").then(|| value.parse::<u64>().ok()).flatten()
}

pub(crate) fn read_load_average() -> Option<[u32; 3]> {
    parse_load_average(&fs::read_to_string("/proc/loadavg").ok()?)
}

pub(crate) fn parse_load_average(input: &str) -> Option<[u32; 3]> {
    let mut fields = input.split_whitespace();
    Some([
        parse_decimal_milli(fields.next()?)?,
        parse_decimal_milli(fields.next()?)?,
        parse_decimal_milli(fields.next()?)?,
    ])
}

pub(crate) fn parse_decimal_milli(value: &str) -> Option<u32> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || fraction.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let whole = whole.parse::<u32>().ok()?.checked_mul(1_000)?;
    let mut fractional = 0_u32;
    let mut scale = 100_u32;
    for digit in fraction.bytes().take(3) {
        fractional = fractional.checked_add(u32::from(digit - b'0') * scale)?;
        scale /= 10;
    }
    whole.checked_add(fractional)
}

pub(crate) fn bytes_per_second(previous: u64, current: u64, elapsed: Duration) -> Option<u64> {
    let delta = current.checked_sub(previous)?;
    let nanos = elapsed.as_nanos();
    if nanos == 0 {
        return None;
    }
    u128::from(delta)
        .checked_mul(1_000_000_000)?
        .checked_div(nanos)
        .and_then(|rate| u64::try_from(rate).ok())
}

pub(crate) fn disk_rates(
    previous: &DiskCounters,
    current: &DiskCounters,
    elapsed: Duration,
) -> Option<(u64, u64)> {
    (previous.major == current.major && previous.minor == current.minor).then_some(())?;
    Some((
        bytes_per_second(previous.read_bytes, current.read_bytes, elapsed)?,
        bytes_per_second(previous.write_bytes, current.write_bytes, elapsed)?,
    ))
}

pub(crate) fn network_rates(
    previous: &NetworkCounters,
    current: &NetworkCounters,
    elapsed: Duration,
) -> Option<(u64, u64)> {
    (previous.interface == current.interface).then_some(())?;
    Some((
        bytes_per_second(previous.receive_bytes, current.receive_bytes, elapsed)?,
        bytes_per_second(previous.transmit_bytes, current.transmit_bytes, elapsed)?,
    ))
}

pub(crate) fn parse_diskstats(input: &str, major: u64, minor: u64) -> Option<DiskCounters> {
    input.lines().find_map(|line| {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 10
            || fields[0].parse::<u64>().ok()? != major
            || fields[1].parse::<u64>().ok()? != minor
        {
            return None;
        }
        Some(DiskCounters {
            major,
            minor,
            read_bytes: fields[5].parse::<u64>().ok()?.checked_mul(512)?,
            write_bytes: fields[9].parse::<u64>().ok()?.checked_mul(512)?,
        })
    })
}

#[cfg(target_os = "linux")]
pub(crate) fn build_disk_device(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let device = fs::metadata(path).ok()?.dev();
    Some((
        u64::from(libc::major(device)),
        u64::from(libc::minor(device)),
    ))
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn build_disk_device(_path: &Path) -> Option<(u64, u64)> {
    None
}

#[cfg(target_os = "linux")]
pub(crate) fn read_disk_counters(major: u64, minor: u64) -> Option<DiskCounters> {
    parse_diskstats(&fs::read_to_string("/proc/diskstats").ok()?, major, minor)
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn read_disk_counters(_major: u64, _minor: u64) -> Option<DiskCounters> {
    None
}

pub(crate) fn parse_default_route_interface(input: &str) -> Option<String> {
    input
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.len() < 8 || fields[1] != "00000000" || fields[7] != "00000000" {
                return None;
            }
            let flags = u16::from_str_radix(fields[3], 16).ok()?;
            if flags & 1 == 0 {
                return None;
            }
            Some((fields[6].parse::<u64>().ok()?, fields[0].to_owned()))
        })
        .min_by_key(|(metric, _)| *metric)
        .map(|(_, interface)| interface)
}

pub(crate) fn parse_network_dev(input: &str, interface: &str) -> Option<NetworkCounters> {
    input.lines().find_map(|line| {
        let (name, counters) = line.split_once(':')?;
        (name.trim() == interface).then_some(())?;
        let fields = counters.split_whitespace().collect::<Vec<_>>();
        (fields.len() >= 16).then_some(NetworkCounters {
            interface: interface.to_owned(),
            receive_bytes: fields[0].parse().ok()?,
            transmit_bytes: fields[8].parse().ok()?,
        })
    })
}

#[cfg(target_os = "linux")]
pub(crate) fn read_default_network_interface() -> Option<String> {
    parse_default_route_interface(&fs::read_to_string("/proc/net/route").ok()?)
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn read_default_network_interface() -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
pub(crate) fn read_network_counters(interface: &str) -> Option<NetworkCounters> {
    parse_network_dev(&fs::read_to_string("/proc/net/dev").ok()?, interface)
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn read_network_counters(_interface: &str) -> Option<NetworkCounters> {
    None
}

#[cfg(unix)]
pub(crate) fn disk_capacity_bytes(path: &Path) -> Option<(u64, u64)> {
    let path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `path` is a NUL-terminated C string and `stat` is valid writable storage.
    if unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: a successful `statvfs` call initializes `stat`.
    let stat = unsafe { stat.assume_init() };
    let available = stat.f_bavail.saturating_mul(stat.f_frsize);
    let total = stat.f_blocks.saturating_mul(stat.f_frsize);
    (total > 0 && available <= total).then_some((available, total))
}

#[cfg(not(unix))]
pub(crate) fn disk_capacity_bytes(_path: &Path) -> Option<(u64, u64)> {
    None
}
