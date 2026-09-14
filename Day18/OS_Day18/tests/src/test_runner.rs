use crate::serial;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed  = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx")  0xf4_u16,
            in("eax") exit_code as u32,
        );
    }
    loop {}
}

pub trait Testable {
    fn run(&self);
}

impl<T: Fn()> Testable for T {
    fn run(&self) {
        serial::print("  [TEST] ");
        serial::print(core::any::type_name::<T>());
        serial::print(" ... ");
        self();
        serial::println("ok");
    }
}

pub fn run_tests(tests: &[&dyn Testable]) -> ! {
    serial::println("\n=============================");
    serial::println(" KERNEL TEST SUITE - Jour 6");
    serial::println("=============================\n");

    for test in tests {
        test.run();
    }

    serial::println("\n=============================");
    serial::println(" Tous les tests ont reussi !");
    serial::println("=============================\n");

    exit_qemu(QemuExitCode::Success)
}
