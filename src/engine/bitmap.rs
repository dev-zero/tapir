#[derive(Debug, Clone)]
pub struct Bitmap1Bit {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Bitmap1Bit {
    pub fn new(width: u32, height: u32) -> Self {
        let row_bytes = (width + 7) / 8;
        Self {
            width,
            height,
            data: vec![0; (row_bytes * height) as usize],
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, value: bool) {
        if x >= self.width || y >= self.height {
            return;
        }
        let row_bytes = (self.width + 7) / 8;
        let byte_idx = (y * row_bytes + x / 8) as usize;
        let bit_idx = 7 - (x % 8);

        if value {
            self.data[byte_idx] |= 1 << bit_idx;
        } else {
            self.data[byte_idx] &= !(1 << bit_idx);
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let row_bytes = (self.width + 7) / 8;
        let byte_idx = (y * row_bytes + x / 8) as usize;
        let bit_idx = 7 - (x % 8);
        (self.data[byte_idx] >> bit_idx) & 1 == 1
    }

    pub fn column_bytes(&self, col: u32) -> Vec<u8> {
        let height_bytes = (self.height + 7) / 8;
        let mut bytes = vec![0u8; height_bytes as usize];

        for row in 0..self.height {
            if self.get_pixel(col, row) {
                let inv = self.height - 1 - row;
                let byte_idx = (inv / 8) as usize;
                let bit_idx = 7 - (inv % 8);
                bytes[byte_idx] |= 1 << bit_idx;
            }
        }

        bytes
    }

    pub fn from_png(data: &[u8]) -> Result<Self, image::ImageError> {
        let img = image::load_from_memory_with_format(data, image::ImageFormat::Png)?;
        let gray = img.to_luma8();
        let (w, h) = gray.dimensions();

        let mut bmp = Bitmap1Bit::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let pixel = gray.get_pixel(x, y).0[0];
                if pixel < 128 {
                    bmp.set_pixel(x, y, true);
                }
            }
        }

        Ok(bmp)
    }

    pub fn to_png(&self) -> Vec<u8> {
        use image::{GrayImage, Luma};
        let mut img = GrayImage::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let val = if self.get_pixel(x, y) { 0u8 } else { 255u8 };
                img.put_pixel(x, y, Luma([val]));
            }
        }
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        image::ImageEncoder::write_image(
            encoder,
            &img,
            self.width,
            self.height,
            image::ColorType::L8.into(),
        )
        .expect("PNG encode failed");
        buf
    }
}
