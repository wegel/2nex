//! UEFI console passphrase input
//!
//! reads passphrase from UEFI console without echo.

use alloc::vec::Vec;
use core::cell::RefCell;
use uefi::proto::console::text::{Key, ScanCode};

const MAX_PASSPHRASE_LEN: usize = 512;

#[derive(Debug)]
#[allow(dead_code)]
pub enum PassphraseError {
    ConsoleError,
    Cancelled,
}

/// input event from keyboard
enum InputEvent {
    Char(char),
    Backspace,
    Enter,
    Escape,
    None,
}

/// read passphrase from UEFI console
///
/// displays prompt, reads input without echo, returns on Enter.
/// supports backspace for editing.
pub fn read_passphrase(prompt: &str) -> Result<Vec<u8>, PassphraseError> {
    log::info!("{}", prompt);
    let mut passphrase: Vec<u8> = Vec::with_capacity(MAX_PASSPHRASE_LEN);
    loop {
        match read_input_event() {
            InputEvent::Enter => {
                log::info!("");
                return Ok(passphrase);
            }
            InputEvent::Escape => return Err(PassphraseError::Cancelled),
            InputEvent::Backspace => {
                passphrase.pop();
            }
            InputEvent::Char(character) => append_character(&mut passphrase, character),
            InputEvent::None => uefi::boot::stall(10_000),
        }
    }
}

fn read_input_event() -> InputEvent {
    let event = RefCell::new(InputEvent::None);
    uefi::system::with_stdin(|stdin| {
        if let Ok(Some(key)) = stdin.read_key() {
            *event.borrow_mut() = match key {
                Key::Printable(character) => printable_event(char::from(character)),
                Key::Special(ScanCode::ESCAPE) => InputEvent::Escape,
                Key::Special(_) => InputEvent::None,
            };
        }
    });
    event.into_inner()
}

fn printable_event(character: char) -> InputEvent {
    match character {
        '\r' | '\n' => InputEvent::Enter,
        '\x08' | '\x7f' => InputEvent::Backspace,
        _ => InputEvent::Char(character),
    }
}

fn append_character(passphrase: &mut Vec<u8>, character: char) {
    if passphrase.len() >= MAX_PASSPHRASE_LEN {
        return;
    }
    let mut buffer = [0u8; 4];
    let encoded = character.encode_utf8(&mut buffer);
    passphrase.extend_from_slice(encoded.as_bytes());
}
