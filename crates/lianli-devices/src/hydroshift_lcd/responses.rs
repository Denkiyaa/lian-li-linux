use super::protocol::{A_HEADER_LEN, INPUT_REPORT_SIZE};
use anyhow::{ensure, Result};
use std::time::{Duration, Instant};

pub(super) struct ResponseReader {
    deadline: Instant,
    remaining_reports: usize,
}

impl ResponseReader {
    pub(super) fn new(timeout_ms: i32) -> Self {
        Self::with_report_limit(timeout_ms, 64)
    }

    pub(super) fn with_report_limit(timeout_ms: i32, remaining_reports: usize) -> Self {
        Self {
            deadline: Instant::now() + Duration::from_millis(timeout_ms.max(1) as u64),
            remaining_reports,
        }
    }

    pub(super) fn expired(&self) -> bool {
        Instant::now() >= self.deadline
    }

    pub(super) fn read_into(
        &mut self,
        bytes: &mut [u8],
        mut read: impl FnMut(&mut [u8], i32) -> Result<usize>,
    ) -> Result<usize> {
        ensure!(
            !lianli_transport::usb::shutting_down(),
            "AIO response read cancelled"
        );
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        ensure!(!remaining.is_zero(), "AIO response deadline expired");
        ensure!(
            self.remaining_reports > 0,
            "AIO response report limit exceeded"
        );
        let n = read(bytes, remaining.as_millis().clamp(1, 100) as i32)?;
        ensure!(n <= bytes.len(), "oversized AIO response");
        if n > 0 {
            self.remaining_reports -= 1;
        }
        Ok(n)
    }

    pub(super) fn read(
        &mut self,
        mut read: impl FnMut(&mut [u8], i32) -> Result<usize>,
    ) -> Result<Vec<u8>> {
        loop {
            let mut bytes = [0; INPUT_REPORT_SIZE];
            let n = self.read_into(&mut bytes, &mut read)?;
            if n == 0 {
                continue;
            }
            ensure!(n >= 2, "short AIO response");
            return Ok(bytes[..n].to_vec());
        }
    }
}

pub(super) fn firmware_text(response: &[u8]) -> Result<String> {
    ensure!(
        response.len() >= A_HEADER_LEN,
        "short firmware response header"
    );
    let length = usize::from(response[5]);
    let data = response
        .get(A_HEADER_LEN..A_HEADER_LEN + length)
        .ok_or_else(|| anyhow::anyhow!("truncated firmware response payload"))?;
    Ok(String::from_utf8_lossy(data)
        .trim_end_matches('\0')
        .to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unrelated_reports_cannot_extend_the_transaction_indefinitely() {
        let mut reader = ResponseReader::new(3000);
        let mut calls = 0;
        loop {
            let result = reader.read(|bytes, timeout| {
                assert!((1..=100).contains(&timeout));
                calls += 1;
                bytes[..2].copy_from_slice(&[1, 0x81]);
                Ok(2)
            });
            if result.is_err() {
                break;
            }
        }
        assert_eq!(calls, 64);
    }

    #[test]
    fn expired_deadline_does_not_start_another_read() {
        let mut reader = ResponseReader {
            deadline: Instant::now(),
            remaining_reports: 64,
        };
        assert!(reader.read(|_, _| panic!("read after deadline")).is_err());
    }

    #[test]
    fn caller_buffer_and_scan_limit_are_preserved() {
        let mut reader = ResponseReader::with_report_limit(1000, 2);
        let mut bytes = [0; 1024];
        for _ in 0..2 {
            let size = reader
                .read_into(&mut bytes, |bytes, timeout| {
                    assert!((1..=100).contains(&timeout));
                    bytes.fill(2);
                    Ok(bytes.len())
                })
                .unwrap();
            assert_eq!(size, 1024);
        }
        assert!(reader
            .read_into(&mut bytes, |_, _| panic!("read past report limit"))
            .is_err());
    }

    #[test]
    fn firmware_payload_uses_actual_report_length() {
        assert!(firmware_text(&[1, 0x86]).is_err());
        assert!(firmware_text(&[1, 0x86, 0, 0, 0, 3, b'1']).is_err());
        assert_eq!(
            firmware_text(&[1, 0x86, 0, 0, 0, 4, b'1', b'.', b'7', 0]).unwrap(),
            "1.7"
        );
    }

    #[test]
    fn one_byte_reply_cannot_inherit_previous_opcode() {
        let mut reader = ResponseReader::new(3000);
        assert!(reader
            .read(|bytes, _| {
                bytes[0] = 1;
                Ok(1)
            })
            .is_err());
    }
}
