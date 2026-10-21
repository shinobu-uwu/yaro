#[cfg(target_arch = "x86_64")]
pub mod x64;

/// The paging architecture selected for the compilation target.
#[cfg(target_arch = "x86_64")]
pub type CurrentArch = x64::X86Arch;
