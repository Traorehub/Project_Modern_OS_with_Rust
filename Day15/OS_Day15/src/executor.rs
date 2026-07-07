//! Jour 14 — Exécuteur de tâches coopératif simple
//! Pas un vrai async/await Rust (nécessiterait alloc + Waker complet)
//! mais le même PRINCIPE : poll en boucle sans bloquer le CPU

use crate::{serial, keyboard, vga, vga2};
use core::fmt::Write as _;

/// Exécuteur simple — une seule "tâche" : afficher les touches clavier
pub struct Executor {
    keyboard_task: keyboard::KeyboardFuture,
    running: bool,
}

impl Executor {
    pub const fn new() -> Self {
        Executor {
            keyboard_task: keyboard::KeyboardFuture,
            running: true,
        }
    }

    /// Boucle principale — multitâche COOPÉRATIF
    /// Chaque tâche doit "rendre la main" volontairement (poll non bloquant)
    /// Contrairement au préemptif où l'OS force le changement de contexte
    pub fn run(&mut self) -> ! {
        serial::println("Executeur demarre — tapez au clavier (QEMU)");
        vga::WRITER.lock().write_fmt(format_args!("Tapez au clavier...\n")).ok();

        let mut iterations: u64 = 0;

        loop {
            // Poll la tâche clavier — non bloquant
            if let Some(ch) = self.keyboard_task.poll() {
                serial::print("Touche : ");
                let mut buf = [0u8; 4];
                let s = ch.encode_utf8(&mut buf);
                serial::print(s);
                serial::println("");

                vga::WRITER.lock().write_fmt(format_args!("{}", ch)).ok();
            }

            // Ici on pourrait poll() d'autres tâches (réseau, timer...)
            // C'est ça le multitâche coopératif : chaque tâche dit
            // "j'ai rien à faire pour l'instant" sans bloquer les autres

            iterations += 1;
            if iterations % 10_000_000 == 0 {
                // Heartbeat optionnel pour montrer que la boucle tourne
            }

            unsafe { core::arch::asm!("hlt"); } // attendre la prochaine interruption
        }
    }
}
