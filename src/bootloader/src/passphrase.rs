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
        // read one key event
        let event = RefCell::new(InputEvent::None);

        uefi::system::with_stdin(|stdin| {
            if let Ok(Some(key)) = stdin.read_key() {
                let e = match key {
                    Key::Printable(c) => {
                        let ch = char::from(c);
                        match ch {
                            '\r' | '\n' => InputEvent::Enter,
                            '\x08' | '\x7f' => InputEvent::Backspace,
                            _ => InputEvent::Char(ch),
                        }
                    }
                    Key::Special(scan) => {
                        if scan == ScanCode::ESCAPE {
                            InputEvent::Escape
                        } else {
                            InputEvent::None
                        }
                    }
                };
                *event.borrow_mut() = e;
            }
        });

        // process the event outside the closure
        match event.into_inner() {
            InputEvent::Enter => {
                log::info!(""); // newline
                return Ok(passphrase);
            }
            InputEvent::Escape => {
                return Err(PassphraseError::Cancelled);
            }
            InputEvent::Backspace => {
                passphrase.pop();
            }
            InputEvent::Char(ch) => {
                if passphrase.len() < MAX_PASSPHRASE_LEN {
                    let mut buf = [0u8; 4];
                    let s = ch.encode_utf8(&mut buf);
                    passphrase.extend_from_slice(s.as_bytes());
                }
            }
            InputEvent::None => {
                // no key, wait a bit
                uefi::boot::stall(10_000); // 10ms
            }
        }
    }
}
