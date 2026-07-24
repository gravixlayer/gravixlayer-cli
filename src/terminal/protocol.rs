// src/terminal/protocol.rs — WebSocket terminal binary frame codec.
//
// Shared terminal frame protocol used by GravixLayer clients.
//
// ALL multi-byte integers are big-endian (network byte order).
//
// ────────────────────────────────────────────────────────────────────────────
// Client → Server frames
// ────────────────────────────────────────────────────────────────────────────
//
//   0x01 + <UTF-8 bytes>              Keyboard / paste input
//   0x02 + cols(u16be) + rows(u16be)  Terminal resize  (5 bytes total)
//   0x03                              Close / disconnect
//
// ────────────────────────────────────────────────────────────────────────────
// Server → Client frames
// ────────────────────────────────────────────────────────────────────────────
//
//   0x01 + <UTF-8 bytes>
//       PTY output to display in the terminal.
//
//   0x02 + pid(u32be) + session_id_len(u16be) + session_id_bytes
//       Session ready: PTY has started.
//
//   0x03 + exit_code(i32be) + status_len(u16be) + status_bytes
//       Process exited.
//
//   0x04 + fatal(u8) + <UTF-8 message bytes>
//       Error: fatal=1 means the connection will be closed.

// ---------------------------------------------------------------------------
// Client → Server frame type bytes
// ---------------------------------------------------------------------------

pub const CLIENT_INPUT: u8 = 0x01;
pub const CLIENT_RESIZE: u8 = 0x02;
pub const CLIENT_CLOSE: u8 = 0x03;

// ---------------------------------------------------------------------------
// Server → Client frame type bytes
// ---------------------------------------------------------------------------

pub const SERVER_OUTPUT: u8 = 0x01;
pub const SERVER_READY: u8 = 0x02;
pub const SERVER_EXIT: u8 = 0x03;
pub const SERVER_ERROR: u8 = 0x04;

// ---------------------------------------------------------------------------
// Encode (Client → Server)
// ---------------------------------------------------------------------------

/// Encode keyboard / paste input as a binary frame.
#[must_use]
pub fn encode_input(data: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(1 + data.len());
    frame.push(CLIENT_INPUT);
    frame.extend_from_slice(data);
    frame
}

/// Encode a terminal resize event as a 5-byte binary frame (big-endian).
///
/// Layout: `[0x02] [cols_hi] [cols_lo] [rows_hi] [rows_lo]`
#[must_use]
pub fn encode_resize(cols: u16, rows: u16) -> Vec<u8> {
    let mut frame = Vec::with_capacity(5);
    frame.push(CLIENT_RESIZE);
    frame.extend_from_slice(&cols.to_be_bytes());
    frame.extend_from_slice(&rows.to_be_bytes());
    frame
}

/// Encode a close frame (single byte).
#[must_use]
pub fn encode_close() -> Vec<u8> {
    vec![CLIENT_CLOSE]
}

// ---------------------------------------------------------------------------
// Decode (Server → Client)
// ---------------------------------------------------------------------------

/// Decoded server message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerFrame {
    /// PTY output bytes to display.
    Output(Vec<u8>),

    /// Session is ready.  `session_id` is an opaque string from the server.
    Ready { pid: u32, session_id: String },

    /// Process exited.
    Exit { exit_code: i32, status: String },

    /// Server-side error.  `fatal=true` → connection will close.
    Error { fatal: bool, message: String },
}

/// Errors that can occur when decoding a server frame.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("empty frame")]
    Empty,

    #[error("unknown frame type: 0x{0:02x}")]
    UnknownFrameType(u8),

    #[error("frame too short for type 0x{frame_type:02x}: need {need} bytes, have {have}")]
    TooShort {
        frame_type: u8,
        need: usize,
        have: usize,
    },

    #[error("invalid UTF-8 in frame: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// Decode a binary message received from the server WebSocket.
pub fn decode_server_frame(data: &[u8]) -> Result<ServerFrame, ProtocolError> {
    if data.is_empty() {
        return Err(ProtocolError::Empty);
    }

    let frame_type = data[0];
    let payload = &data[1..];

    match frame_type {
        SERVER_OUTPUT => Ok(ServerFrame::Output(payload.to_vec())),

        SERVER_READY => {
            // pid(u32be)=4 + session_id_len(u16be)=2 + session_id_bytes=N
            const MIN: usize = 4 + 2;
            if payload.len() < MIN {
                return Err(ProtocolError::TooShort {
                    frame_type,
                    need: MIN,
                    have: payload.len(),
                });
            }
            let pid = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
            let sid_len = u16::from_be_bytes([payload[4], payload[5]]) as usize;
            let sid_end = 6 + sid_len;
            if payload.len() < sid_end {
                return Err(ProtocolError::TooShort {
                    frame_type,
                    need: sid_end,
                    have: payload.len(),
                });
            }
            let session_id = String::from_utf8(payload[6..sid_end].to_vec())?;
            Ok(ServerFrame::Ready { pid, session_id })
        }

        SERVER_EXIT => {
            // exit_code(i32be)=4 + status_len(u16be)=2 + status_bytes=N
            const MIN: usize = 4 + 2;
            if payload.len() < MIN {
                return Err(ProtocolError::TooShort {
                    frame_type,
                    need: MIN,
                    have: payload.len(),
                });
            }
            let exit_code = i32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
            let status_len = u16::from_be_bytes([payload[4], payload[5]]) as usize;
            let status_end = 6 + status_len;
            if payload.len() < status_end {
                return Err(ProtocolError::TooShort {
                    frame_type,
                    need: status_end,
                    have: payload.len(),
                });
            }
            let status = String::from_utf8(payload[6..status_end].to_vec())?;
            Ok(ServerFrame::Exit { exit_code, status })
        }

        SERVER_ERROR => {
            // fatal(u8)=1 + message bytes
            if payload.is_empty() {
                return Err(ProtocolError::TooShort {
                    frame_type,
                    need: 1,
                    have: 0,
                });
            }
            let fatal = payload[0] != 0;
            let message = String::from_utf8(payload[1..].to_vec())?;
            Ok(ServerFrame::Error { fatal, message })
        }

        other => Err(ProtocolError::UnknownFrameType(other)),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Encode ──────────────────────────────────────────────────────────────

    #[test]
    fn encode_input_prepends_type_byte() {
        let frame = encode_input(b"hello");
        assert_eq!(frame[0], CLIENT_INPUT);
        assert_eq!(&frame[1..], b"hello");
    }

    #[test]
    fn encode_resize_is_big_endian() {
        let frame = encode_resize(120, 40);
        assert_eq!(frame.len(), 5);
        assert_eq!(frame[0], CLIENT_RESIZE);
        // cols = 120 = 0x0078
        assert_eq!(frame[1], 0x00);
        assert_eq!(frame[2], 0x78);
        // rows = 40 = 0x0028
        assert_eq!(frame[3], 0x00);
        assert_eq!(frame[4], 0x28);
    }

    #[test]
    fn encode_resize_max_values() {
        let frame = encode_resize(u16::MAX, u16::MAX);
        assert_eq!(&frame[1..3], &[0xFF, 0xFF]);
        assert_eq!(&frame[3..5], &[0xFF, 0xFF]);
    }

    #[test]
    fn encode_close_is_single_byte() {
        let frame = encode_close();
        assert_eq!(frame, vec![CLIENT_CLOSE]);
    }

    // ── Decode: Output ───────────────────────────────────────────────────────

    #[test]
    fn decode_output_frame() {
        let mut data = vec![SERVER_OUTPUT];
        data.extend_from_slice(b"hello world");
        let frame = decode_server_frame(&data).unwrap();
        assert_eq!(frame, ServerFrame::Output(b"hello world".to_vec()));
    }

    #[test]
    fn decode_output_empty_payload() {
        let data = vec![SERVER_OUTPUT];
        let frame = decode_server_frame(&data).unwrap();
        assert_eq!(frame, ServerFrame::Output(vec![]));
    }

    // ── Decode: Ready ────────────────────────────────────────────────────────

    #[test]
    fn decode_ready_frame() {
        let session = "sess-abc-123";
        let mut payload = vec![SERVER_READY];
        payload.extend_from_slice(&42u32.to_be_bytes()); // pid
        payload.extend_from_slice(&(session.len() as u16).to_be_bytes()); // session_id_len
        payload.extend_from_slice(session.as_bytes());

        let frame = decode_server_frame(&payload).unwrap();
        assert_eq!(
            frame,
            ServerFrame::Ready {
                pid: 42,
                session_id: "sess-abc-123".into()
            }
        );
    }

    #[test]
    fn decode_ready_too_short_returns_error() {
        let data = vec![SERVER_READY, 0x00, 0x00]; // only 3 bytes of payload, need 6
        assert!(matches!(
            decode_server_frame(&data),
            Err(ProtocolError::TooShort { .. })
        ));
    }

    // ── Decode: Exit ─────────────────────────────────────────────────────────

    #[test]
    fn decode_exit_frame_success() {
        let status = "exited";
        let mut payload = vec![SERVER_EXIT];
        payload.extend_from_slice(&0i32.to_be_bytes()); // exit_code = 0
        payload.extend_from_slice(&(status.len() as u16).to_be_bytes());
        payload.extend_from_slice(status.as_bytes());

        let frame = decode_server_frame(&payload).unwrap();
        assert_eq!(
            frame,
            ServerFrame::Exit {
                exit_code: 0,
                status: "exited".into()
            }
        );
    }

    #[test]
    fn decode_exit_frame_nonzero_exit_code() {
        let status = "killed";
        let mut payload = vec![SERVER_EXIT];
        payload.extend_from_slice(&(-1i32).to_be_bytes());
        payload.extend_from_slice(&(status.len() as u16).to_be_bytes());
        payload.extend_from_slice(status.as_bytes());

        let frame = decode_server_frame(&payload).unwrap();
        assert_eq!(
            frame,
            ServerFrame::Exit {
                exit_code: -1,
                status: "killed".into()
            }
        );
    }

    // ── Decode: Error ────────────────────────────────────────────────────────

    #[test]
    fn decode_error_frame_fatal() {
        let msg = "connection reset";
        let mut payload = vec![SERVER_ERROR, 1u8]; // fatal = 1
        payload.extend_from_slice(msg.as_bytes());

        let frame = decode_server_frame(&payload).unwrap();
        assert_eq!(
            frame,
            ServerFrame::Error {
                fatal: true,
                message: "connection reset".into()
            }
        );
    }

    #[test]
    fn decode_error_frame_non_fatal() {
        let msg = "heartbeat missed";
        let mut payload = vec![SERVER_ERROR, 0u8]; // fatal = 0
        payload.extend_from_slice(msg.as_bytes());

        let frame = decode_server_frame(&payload).unwrap();
        assert_eq!(
            frame,
            ServerFrame::Error {
                fatal: false,
                message: "heartbeat missed".into()
            }
        );
    }

    // ── Decode: Edge cases ───────────────────────────────────────────────────

    #[test]
    fn decode_empty_returns_error() {
        assert!(matches!(
            decode_server_frame(&[]),
            Err(ProtocolError::Empty)
        ));
    }

    #[test]
    fn decode_unknown_frame_type() {
        assert!(matches!(
            decode_server_frame(&[0xFF, 0x00]),
            Err(ProtocolError::UnknownFrameType(0xFF))
        ));
    }
}
