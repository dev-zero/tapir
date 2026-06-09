use crate::engine::bitmap::Bitmap1Bit;

const ESC: u8 = 0x1B;
const SYN: u8 = 0x16;

/// Distance between print head and cutter: 8.1mm at 180 DPI ≈ 57 pixels.
/// Derived from labelle's LABELER_DISTANCE_BETWEEN_PRINT_HEAD_AND_CUTTER_MM.
pub const FEED_LINES_FOR_CUT: u16 = 57;

pub struct PrintJob {
    pub bytes_per_line: u8,
    pub tape_color_id: u8,
    pub synwait: u16,
}

impl PrintJob {
    pub fn encode(&self, bitmap: &Bitmap1Bit) -> Vec<u8> {
        let mut buf = Vec::new();

        buf.extend_from_slice(&[ESC, b'C', self.tape_color_id]);
        buf.extend_from_slice(&[ESC, b'D', self.bytes_per_line]);

        let mut syn_count: u16 = 0;
        let bpl = self.bytes_per_line as usize;
        for col_idx in 0..bitmap.width {
            let col_bytes = bitmap.column_bytes(col_idx);

            buf.push(SYN);
            if col_bytes.len() >= bpl {
                buf.extend_from_slice(&col_bytes[..bpl]);
            } else {
                buf.extend_from_slice(&col_bytes);
                buf.resize(buf.len() + bpl - col_bytes.len(), 0);
            }

            syn_count += 1;
            if syn_count >= self.synwait {
                buf.extend_from_slice(&[ESC, b'A']);
                syn_count = 0;
            }
        }

        buf.extend_from_slice(&[ESC, b'A']);
        buf
    }

    pub fn encode_feed(&self, lines: u16) -> Vec<u8> {
        let mut buf = Vec::new();
        let blank_line = vec![0u8; self.bytes_per_line as usize];

        buf.extend_from_slice(&[ESC, b'C', self.tape_color_id]);
        buf.extend_from_slice(&[ESC, b'D', self.bytes_per_line]);

        let mut syn_count: u16 = 0;
        for _ in 0..lines {
            buf.push(SYN);
            buf.extend_from_slice(&blank_line);

            syn_count += 1;
            if syn_count >= self.synwait {
                buf.extend_from_slice(&[ESC, b'A']);
                syn_count = 0;
            }
        }

        buf.extend_from_slice(&[ESC, b'A']);
        buf
    }
}
