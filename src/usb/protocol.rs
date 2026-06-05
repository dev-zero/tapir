use crate::engine::bitmap::Bitmap1Bit;

const ESC: u8 = 0x1B;
const SYN: u8 = 0x16;

pub struct PrintJob {
    pub bytes_per_line: u8,
    pub tape_color_id: u8,
    pub synwait: u16,
}

impl PrintJob {
    pub fn encode(&self, bitmap: &Bitmap1Bit) -> Vec<u8> {
        let rotated = bitmap.rotate_270();
        let mut buf = Vec::new();

        buf.extend_from_slice(&[ESC, b'C', self.tape_color_id]);
        buf.extend_from_slice(&[ESC, b'D', self.bytes_per_line]);

        let mut syn_count: u16 = 0;
        for col_idx in 0..rotated.width {
            let row_bytes = rotated.column_bytes(col_idx);

            buf.push(SYN);
            buf.extend_from_slice(&row_bytes);

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
