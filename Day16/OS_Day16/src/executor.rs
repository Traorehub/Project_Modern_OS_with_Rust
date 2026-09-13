//! Jour 14 — Exécuteur de tâches coopératif simple
//! Jour 16 — deux sources d'evenements : IRQ0 (timer) et IRQ1 (clavier)
//!
//! Le noyau n'est plus reveille uniquement par le clavier : le timer lui
//! donne un pouls independant. C'est le prerequis du scheduling preemptif.
//!
//! Note taille : aucun entier n'est affiche via `core::fmt` ({} sur un nombre).
//! Le moteur de formatage d'entiers pese plusieurs Ko et `boot.asm` ne charge
//! que 60 secteurs — au-dela, la fin du kernel n'est jamais lue depuis le disque.

use crate::{serial, keyboard, timer, vga, vga2};
use core::fmt::Write as _;

/// Ligne de texte a taille fixe, construite sans allocation ni `core::fmt`.
struct FixedLine {
    buf: [u8; vga2::VGA_WIDTH],
    len: usize,
}

impl FixedLine {
    const fn new() -> Self {
        FixedLine { buf: [b' '; vga2::VGA_WIDTH], len: 0 }
    }

    fn push_str(&mut self, s: &str) {
        for &b in s.as_bytes() {
            if self.len < self.buf.len() {
                self.buf[self.len] = b;
                self.len += 1;
            }
        }
    }

    fn push_dec(&mut self, mut value: u64) {
        let mut tmp = [0u8; 20];
        let mut i = tmp.len();

        if value == 0 {
            i -= 1;
            tmp[i] = b'0';
        }
        while value > 0 {
            i -= 1;
            tmp[i] = b'0' + (value % 10) as u8;
            value /= 10;
        }

        for k in i..tmp.len() {
            if self.len < self.buf.len() {
                self.buf[self.len] = tmp[k];
                self.len += 1;
            }
        }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

/// Exécuteur simple — deux "tâches" : le timer et le clavier
pub struct Executor {
    keyboard_task: keyboard::KeyboardFuture,
    keys_typed: u64,
    last_second: u64,
}

impl Executor {
    pub const fn new() -> Self {
        Executor {
            keyboard_task: keyboard::KeyboardFuture,
            keys_typed: 0,
            last_second: u64::MAX, // force un premier affichage immediat
        }
    }

    /// Barre d'etat sur la derniere ligne VGA.
    ///
    /// On ecrit a une position fixe via `vga::write_str` au lieu d'utiliser le
    /// `Writer` : ca evite de deplacer le curseur et de faire defiler la zone
    /// de saisie a chaque seconde.
    fn draw_status_bar(&self, seconds: u64, ticks: u64) {
        let mut line = FixedLine::new();
        line.push_str(" Uptime ");
        line.push_dec(seconds);
        line.push_str(" s | ");
        line.push_dec(ticks);
        line.push_str(" ticks | ");
        line.push_dec(self.keys_typed);
        line.push_str(" touches ");

        unsafe {
            vga::write_str(
                vga2::VGA_HEIGHT - 1,
                0,
                line.as_str(),
                vga2::make_color(vga2::Color::Black, vga2::Color::LightGray),
            );
        }
    }

    /// Boucle principale — dirigee par les interruptions
    ///
    /// Jour 14 : `hlt` ne se reveillait qu'a une frappe clavier.
    /// Jour 16 : `hlt` se reveille aussi 100 fois par seconde grace a IRQ0,
    /// donc le noyau "vit" meme sans interaction utilisateur.
    pub fn run(&mut self) -> ! {
        serial::println("Executeur demarre — timer IRQ0 + clavier IRQ1");
        vga::WRITER
            .lock()
            .write_fmt(format_args!("Tapez au clavier (l'uptime defile en bas)...\n"))
            .ok();

        loop {
            // ── Tache 1 : le timer ────────────────────────────────────────────
            let seconds = timer::uptime_seconds();
            if seconds != self.last_second {
                self.last_second = seconds;
                let ticks = timer::ticks();

                serial::print("[uptime] ");
                serial::print_dec(seconds);
                serial::print(" s - ");
                serial::print_dec(ticks);
                serial::print(" ticks - ");
                serial::print_dec(self.keys_typed);
                serial::println(" touches");

                self.draw_status_bar(seconds, ticks);
            }

            // ── Tache 2 : le clavier ──────────────────────────────────────────
            if let Some(ch) = self.keyboard_task.poll() {
                self.keys_typed += 1;

                let mut utf8 = [0u8; 4];
                serial::print("Touche : ");
                serial::print(ch.encode_utf8(&mut utf8));
                serial::print(" (t=");
                serial::print_dec(timer::uptime_ms());
                serial::println(" ms)");

                vga::WRITER.lock().write_byte(ch as u8);
            }

            // Veille jusqu'a la prochaine interruption (timer OU clavier).
            // Au pire 10 ms d'attente, contre une attente infinie au Jour 14.
            unsafe { core::arch::asm!("hlt"); }
        }
    }
}
