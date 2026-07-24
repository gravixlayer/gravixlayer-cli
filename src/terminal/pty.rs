// src/terminal/pty.rs — PTY / terminal management for interactive shell sessions.
//
// This module owns the terminal lifecycle for `gravixlayer runtime shell`:
//
//   1. Put the local terminal into raw mode (crossterm)
//   2. Capture the initial window size and send a RESIZE frame
//   3. Spawn two concurrent tasks:
//        a. stdin reader  → encodes as INPUT frames → sends over WebSocket
//        b. WebSocket     → receives server frames  → decoded + written to stdout
//   4. Relay SIGWINCH (Unix) or terminal resize events (Windows / crossterm) as
//      RESIZE frames
//   5. Restore the terminal to normal mode on exit (even after panic)
//
// The WebSocket connection itself is established by the calling command handler
// (`cmd/runtime/shell.rs`) and passed in as a `tokio_tungstenite::WebSocketStream`.

use std::io::{self, Write};

use crossterm::{
    event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{
        self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use super::protocol::{
    decode_server_frame, encode_close, encode_input, encode_resize, ServerFrame,
};

/// Result of running an interactive terminal session.
#[derive(Debug)]
pub struct SessionResult {
    /// Exit code reported by the server, if known.
    pub exit_code: Option<i32>,
    /// Terminal status string reported by the server.
    pub status: String,
}

/// Drive an interactive terminal session over a WebSocket.
///
/// # Arguments
/// - `ws` — established WebSocket stream to the terminal endpoint
/// - `initial_cols` / `initial_rows` — terminal dimensions at session start
///
/// The local terminal is put into raw mode for the duration.  Ctrl-C is
/// forwarded to the remote PTY (not handled locally) so the user can cancel
/// remote processes naturally.  The session ends when:
///   • The server sends an EXIT or fatal ERROR frame
///   • The WebSocket connection closes
pub async fn run_session<S>(
    mut ws: WebSocketStream<S>,
    initial_cols: u16,
    initial_rows: u16,
) -> anyhow::Result<SessionResult>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    // -- Enter raw mode and alternate screen ---------------------------------
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    // RAII guard: restore terminal on any exit path.
    let _guard = RawModeGuard;

    // -- Send initial resize frame -------------------------------------------
    ws.send(Message::Binary(
        encode_resize(initial_cols, initial_rows).into(),
    ))
    .await?;

    // -- Event loop ----------------------------------------------------------
    let mut event_stream = EventStream::new();
    let mut result = SessionResult {
        exit_code: None,
        status: String::new(),
    };

    loop {
        tokio::select! {
            // ── Local terminal event (keyboard / resize) ──────────────────
            event = event_stream.next() => {
                match event {
                    None => break, // stdin closed
                    Some(Err(e)) => {
                        tracing::warn!("terminal event error: {e}");
                        break;
                    }
                    Some(Ok(Event::Key(key))) => {
                        let bytes = key_event_to_bytes(key);
                        if !bytes.is_empty() {
                            ws.send(Message::Binary(encode_input(&bytes).into())).await?;
                        }
                    }
                    Some(Ok(Event::Resize(cols, rows))) => {
                        ws.send(Message::Binary(encode_resize(cols, rows).into())).await?;
                    }
                    Some(Ok(_)) => {} // mouse events etc.
                }
            }

            // ── WebSocket message from server ─────────────────────────────
            msg = ws.next() => {
                match msg {
                    None => break, // connection closed
                    Some(Err(e)) => {
                        tracing::warn!("WebSocket error: {e}");
                        break;
                    }
                    Some(Ok(Message::Binary(data))) => {
                        match decode_server_frame(&data) {
                            Ok(ServerFrame::Output(bytes)) => {
                                stdout.write_all(&bytes)?;
                                stdout.flush()?;
                            }
                            Ok(ServerFrame::Ready { pid, session_id }) => {
                                tracing::debug!(pid, session_id, "terminal session ready");
                            }
                            Ok(ServerFrame::Exit { exit_code, status }) => {
                                result.exit_code = Some(exit_code);
                                result.status = status;
                                break;
                            }
                            Ok(ServerFrame::Error { fatal, message }) => {
                                tracing::warn!(?fatal, message, "terminal error from server");
                                if fatal {
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::warn!("failed to decode frame: {e}");
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Ok(_)) => {} // text frames / ping/pong — ignore
                }
            }
        }
    }

    // Send CLOSE frame (best-effort)
    let _ = ws.send(Message::Binary(encode_close().into())).await;
    let _ = ws.close(None).await;

    Ok(result)
}

// ---------------------------------------------------------------------------
// Key event → raw bytes
// ---------------------------------------------------------------------------

/// Convert a crossterm `KeyEvent` into the byte sequence to send to the PTY.
///
/// Special keys are encoded as ANSI escape sequences so standard shell
/// readline / vi key bindings work correctly.
fn key_event_to_bytes(key: KeyEvent) -> Vec<u8> {
    use KeyCode::*;
    match (key.modifiers, key.code) {
        // Ctrl-C, Ctrl-D, Ctrl-Z, etc. → control codes
        (KeyModifiers::CONTROL, Char(c)) => {
            let byte = (c as u8).wrapping_sub(b'a').wrapping_add(1);
            vec![byte]
        }
        (_, Enter) => vec![b'\r'],
        (_, Backspace) => vec![0x7f],
        (_, Delete) => vec![0x1b, b'[', b'3', b'~'],
        (_, Tab) => vec![b'\t'],
        (_, Esc) => vec![0x1b],
        (_, Up) => vec![0x1b, b'[', b'A'],
        (_, Down) => vec![0x1b, b'[', b'B'],
        (_, Right) => vec![0x1b, b'[', b'C'],
        (_, Left) => vec![0x1b, b'[', b'D'],
        (_, Home) => vec![0x1b, b'[', b'H'],
        (_, End) => vec![0x1b, b'[', b'F'],
        (_, PageUp) => vec![0x1b, b'[', b'5', b'~'],
        (_, PageDown) => vec![0x1b, b'[', b'6', b'~'],
        (_, F(1)) => vec![0x1b, b'O', b'P'],
        (_, F(2)) => vec![0x1b, b'O', b'Q'],
        (_, F(3)) => vec![0x1b, b'O', b'R'],
        (_, F(4)) => vec![0x1b, b'O', b'S'],
        (_, F(n)) if (5..=8).contains(&n) => {
            vec![0x1b, b'[', b'1', b'5' + (n - 5), b'~']
        }
        (_, F(n)) if (9..=12).contains(&n) => {
            vec![0x1b, b'[', b'2', b'0' + (n - 9), b'~']
        }
        (_, Char(c)) => {
            let mut buf = [0u8; 4];
            c.encode_utf8(&mut buf).as_bytes().to_vec()
        }
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// RAII guard: restore terminal on drop
// ---------------------------------------------------------------------------

struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

// ---------------------------------------------------------------------------
// Terminal size helper
// ---------------------------------------------------------------------------

/// Return the current terminal dimensions `(cols, rows)`.
///
/// Falls back to 80×24 if the terminal size cannot be queried.
pub fn terminal_size() -> (u16, u16) {
    terminal::size().unwrap_or((80, 24))
}
