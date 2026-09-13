// Module port série — utilisé pour les tests et le debug
// COM1 = 0x3F8, vitesse 115200 baud

const COM1: u16 = 0x3F8;

/// Envoie un octet sur COM1
unsafe fn write_byte(byte: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") COM1,
            in("al") byte,
        );
    }
}

/// Envoie une chaîne sur COM1
pub fn print(s: &str) {
    for byte in s.bytes() {
        unsafe { write_byte(byte); }
    }
}

/// Envoie une chaîne suivie d'un retour à la ligne
pub fn println(s: &str) {
    print(s);
    unsafe { write_byte(b'\n'); }
}

use core::fmt::{self, Write};

/// Écrit un `fmt::Arguments` sur le port série sans allocation
pub fn print_fmt(args: fmt::Arguments) {
    struct Writer;
    impl fmt::Write for Writer {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            crate::serial::print(s);
            Ok(())
        }
    }
    let mut writer = Writer;
    let _ = writer.write_fmt(args);
}

/// Macro serial_print! — affiche sur le port série
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => ({
        $crate::serial::print_fmt(core::format_args!($($arg)*));
    });
}

/// Macro serial_println! — affiche sur le port série avec newline
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial::println(""));
    ($s:expr) => ($crate::serial::println($s));
    ($($arg:tt)*) => ({
        $crate::serial::print_fmt(core::format_args!("{}\n", format_args!($($arg)*)));
    });
}
