// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn parse_cpu_mask_lists_indices_and_ranges() {
    let cpus: Vec<usize> = parse_cpu_mask("0-3,5,7-8").expect("valid CPU mask");
    assert_eq!(cpus, vec![0, 1, 2, 3, 5, 7, 8]);
}

#[test]
fn parse_cpu_mask_ignores_whitespace() {
    let cpus: Vec<usize> = parse_cpu_mask(" 1 , 2 - 3 ").expect("valid CPU mask");
    assert_eq!(cpus, vec![1, 2, 3]);
}

#[test]
fn parse_cpu_mask_accepts_highest_cpu() {
    let mask: String = format!("{}", MAX_CPUS - 1);
    let cpus: Vec<usize> = parse_cpu_mask(&mask).expect("valid CPU mask");
    assert_eq!(cpus, vec![MAX_CPUS - 1]);
}

#[test]
fn parse_cpu_mask_rejects_out_of_range_cpu() {
    for mask in [format!("{MAX_CPUS}"), format!("0-{MAX_CPUS}")] {
        let error: ::anyhow::Error = parse_cpu_mask(&mask).expect_err("out-of-range CPU mask");
        assert!(error.to_string().contains("out of range"), "unexpected error: {error}");
    }
}

#[test]
fn parse_cpu_mask_rejects_reversed_range() {
    let error: ::anyhow::Error = parse_cpu_mask("3-1").expect_err("reversed CPU range");
    assert!(error.to_string().contains("\"3-1\""), "unexpected error: {error}");
}

#[test]
fn parse_cpu_mask_rejects_malformed_entries() {
    for mask in [
        "", " ", "a", "1,", ",1", "1,,2", "1-", "-1", "1-2-3", "0x1", "1.5",
    ] {
        assert!(parse_cpu_mask(mask).is_err(), "CPU mask {mask:?} should be rejected");
    }
}

#[test]
fn pin_main_thread_rejects_malformed_mask() {
    let error: ::anyhow::Error =
        pin_main_thread(format!("{MAX_CPUS}")).expect_err("out-of-range CPU mask");
    assert!(error.to_string().contains("out of range"), "unexpected error: {error}");
}

#[cfg(unix)]
#[test]
fn pin_main_thread_pins_calling_thread() {
    let allowed_cpus: Vec<usize> = current_thread_cpus();
    let cpu: usize = *allowed_cpus
        .first()
        .expect("calling thread may not run on any CPU");

    pin_main_thread(cpu.to_string()).expect("failed to pin calling thread");
    assert_eq!(current_thread_cpus(), vec![cpu]);

    let allowed_mask: Vec<String> = allowed_cpus.iter().map(usize::to_string).collect();
    pin_main_thread(allowed_mask.join(",")).expect("failed to restore CPU affinity");
    assert_eq!(current_thread_cpus(), allowed_cpus);
}

///
/// Returns the indices of the CPUs on which the calling thread may run.
///
#[cfg(unix)]
fn current_thread_cpus() -> Vec<usize> {
    // SAFETY: `cpu_set_t` is a plain bit array, for which all zeroes is a valid (empty) value.
    let mut set: cpu_set_t = unsafe { mem::zeroed() };
    // SAFETY: `set` is a valid CPU set of the given size, and pid 0 refers to the calling thread.
    let ret: ::libc::c_int =
        unsafe { ::libc::sched_getaffinity(0, mem::size_of::<cpu_set_t>(), &mut set) };
    assert_eq!(ret, 0, "sched_getaffinity() failed: {}", ::std::io::Error::last_os_error());
    // SAFETY: `set` is a valid CPU set, and every CPU index is less than `MAX_CPUS`, which fits in
    // the CPU set.
    (0..MAX_CPUS)
        .filter(|cpu| unsafe { ::libc::CPU_ISSET(*cpu, &set) })
        .collect()
}
