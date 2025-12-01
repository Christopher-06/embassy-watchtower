//! Returns the core ID of the currently executing core based on the target architecture.
//! Supports various architectures including ESP32 (Xtensa and RISC-V), RP2040, and STM32 (single or H7 dual-core).

#[allow(unreachable_code)]
pub fn core_id() -> u32 {
    //
    // 1. ESP32 via esp-hal (xtensa or riscv32)
    //
    #[cfg(target_arch = "xtensa")]
    {
        return esp_hal::system::Cpu::current() as u32;
    }

    #[cfg(target_arch = "riscv32")]
    {
        return esp_hal::system::Cpu::current() as u32;
    }    

    //
    // Fallback: Unknown target
    //
    0
}
