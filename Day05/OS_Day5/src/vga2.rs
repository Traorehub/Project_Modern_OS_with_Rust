//! VGA text-mode driver - Jour 5 (vga2)
//! Safe Wrapper + support fmt::Write
#![allow(dead_code)]

use core::fmt;
use crate::vga1::Mutex;

// Adresse fixe du buffer VGA
const VGA_BUFFER: *mut u8 = 0xb8000 as *mut u8;
const VGA_WIDTH:  usize = 80;
const VGA_HEIGHT: usize = 25;

// ─── Couleurs ────────────────────────────────────────────────────────────────

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Color {
    Black      = 0,  Blue    = 1,  Green     = 2,  Cyan      = 3,
    Red        = 4,  Magenta = 5,  Brown     = 6,  LightGray = 7,
    DarkGray   = 8,  LightBlue = 9, LightGreen = 10, LightCyan = 11,
    LightRed   = 12, Pink    = 13, Yellow    = 14, White     = 15,
}

pub const fn make_color(fg: Color, bg: Color) -> u8 {
    (bg as u8) << 4 | (fg as u8)
}

// ─── Writer ──────────────────────────────────────────────────────────────────

/// Gère l'état complet de l'écran :
/// - position courante du curseur (row, col)
/// - couleur par défaut
/// - accès sécurisé au buffer VGA
pub struct Writer {
    col:   usize,   // colonne courante
    row:   usize,   // ligne courante
    color: u8,      // couleur par défaut
}

impl Writer {
    /// Crée un Writer avec couleur par défaut blanc sur noir
    pub const fn new() -> Self {
        Writer {
            col:   0,
            row:   0,
            color: make_color(Color::White, Color::Black),
        }
    }

    /// Change la couleur courante du Writer
    pub fn set_color(&mut self, fg: Color, bg: Color) {
        self.color = make_color(fg, bg);
    }

    /// Efface tout l'écran
    pub fn clear(&mut self) {
        for row in 0..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                self.write_cell(row, col, b' ', self.color);
            }
        }
        self.row = 0;
        self.col = 0;
    }

    /// Écrit un octet (caractère) à la position courante
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.newline(),
            byte  => {
                // Si on dépasse la largeur → nouvelle ligne automatique
                if self.col >= VGA_WIDTH {
                    self.newline();
                }
                self.write_cell(self.row, self.col, byte, self.color);
                self.col += 1;
            }
        }
    }

    /// Écrit une chaîne de caractères
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // Caractère ASCII imprimable ou newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // Caractère non imprimable → affiche '■'
                _ => self.write_byte(0xfe),
            }
        }
    }

    /// Passe à la ligne suivante avec scroll si nécessaire
    fn newline(&mut self) {
        self.col = 0;
        if self.row < VGA_HEIGHT - 1 {
            self.row += 1;
        } else {
            // Scroll : décale toutes les lignes vers le haut
            self.scroll();
        }
    }

    /// Fait défiler l'écran d'une ligne vers le haut
    fn scroll(&mut self) {
        // Copie chaque ligne vers la ligne au dessus
        for row in 1..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                let offset_src = (row * VGA_WIDTH + col) * 2;
                let offset_dst = ((row - 1) * VGA_WIDTH + col) * 2;
                unsafe {
                    let ch    = core::ptr::read_volatile(VGA_BUFFER.add(offset_src));
                    let color = core::ptr::read_volatile(VGA_BUFFER.add(offset_src + 1));
                    core::ptr::write_volatile(VGA_BUFFER.add(offset_dst),     ch);
                    core::ptr::write_volatile(VGA_BUFFER.add(offset_dst + 1), color);
                }
            }
        }
        // Efface la dernière ligne
        for col in 0..VGA_WIDTH {
            self.write_cell(VGA_HEIGHT - 1, col, b' ', self.color);
        }
        self.row = VGA_HEIGHT - 1;
    }

    /// Écrit directement une cellule VGA (caractère + couleur)
    /// C'est le SEUL endroit où on touche au pointeur brut
    fn write_cell(&self, row: usize, col: usize, ascii: u8, color: u8) {
        // Bounds check - protection contre la corruption mémoire
        if row >= VGA_HEIGHT || col >= VGA_WIDTH {
            return;
        }
        let offset = (row * VGA_WIDTH + col) * 2;
        unsafe {
            core::ptr::write_volatile(VGA_BUFFER.add(offset),     ascii);
            core::ptr::write_volatile(VGA_BUFFER.add(offset + 1), color);
        }
    }
}

// ─── fmt::Write ──────────────────────────────────────────────────────────────

/// Implémente le trait fmt::Write pour pouvoir utiliser write!() et println!()
/// C'est ce trait qui permet à la macro println! de fonctionner
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

// ─── Mutex global ────────────────────────────────────────────────────────────

/// Instance globale unique du Writer protégée par un Mutex spinlock
/// 'static : vit toute la durée du programme
/// Mutex   : un seul accès à la fois, même avec des interruptions
pub static WRITER: Mutex<Writer> = Mutex::new(Writer::new());

// ─── Macros publiques ────────────────────────────────────────────────────────

/// Macro print! pour le VGA - fonctionne comme en Rust standard
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        // lock() prend le Mutex - bloque si quelqu'un d'autre écrit
        $crate::vga2::WRITER.lock().write_fmt(format_args!($($arg)*)).unwrap();
    });
}

/// Macro println! pour le VGA - ajoute un retour à la ligne
#[macro_export]
macro_rules! println {
    ()            => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
