// Infrastructure de test bare-metal
// Gère l'exécution des tests et la communication du résultat à QEMU

use crate::serial;

/// Codes de sortie QEMU via isa-debug-exit
/// QEMU retourne (valeur << 1) | 1
/// donc 0x10 → exit code 33 (succès pour cargo test)
///      0x11 → exit code 35 (échec)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed  = 0x11,
}

/// Envoie le code de sortie à QEMU via le port 0xf4
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

/// Trait que chaque test doit implémenter
pub trait Testable {
    fn run(&self);
}

/// Implémente Testable pour toute fonction fn() -> ()
impl<T: Fn()> Testable for T {
    fn run(&self) {
        // Affiche le nom du test
        serial::print("  [TEST] ");
        serial::print(core::any::type_name::<T>());
        serial::print(" ... ");
        // Exécute le test
        self();
        // Si on arrive ici, le test a réussi (pas de panic)
        serial::println("ok");
    }
}

/// Lanceur de tests - appelé avec la liste de tous les tests
pub fn run_tests(tests: &[&dyn Testable]) {
    serial::println("\n=============================");
    serial::println(" KERNEL TEST SUITE - Jour 6");
    serial::println("=============================\n");

    let count = tests.len();
    serial::print("Lancement de ");
    serial::print(num_to_str(count as u32));
    serial::println(" test(s)...\n");

    for test in tests {
        test.run();
    }

    serial::println("\n=============================");
    serial::println(" Tous les tests ont reussi !");
    serial::println("=============================\n");

    exit_qemu(QemuExitCode::Success);
}

/// Convertit un u32 en str statique pour éviter alloc
fn num_to_str(n: u32) -> &'static str {
    match n {
        0  => "0",  1  => "1",  2  => "2",  3  => "3",
        4  => "4",  5  => "5",  6  => "6",  7  => "7",
        8  => "8",  9  => "9",  10 => "10", 11 => "11",
        12 => "12", 13 => "13", 14 => "14", 15 => "15",
        _  => "?",
    }
}
