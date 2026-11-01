//! Independently implemented bounded readers. No graphics or private data dependencies.
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Truncated { offset: usize, length: usize },
    InvalidDimensions,
    UnexpectedTrailingData,
    TooManyFrames,
}
impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DecodeError {}

#[derive(Debug, PartialEq, Eq)]
pub struct IndexedFrame<'a> {
    pub width: u8,
    pub height: u8,
    pub pixels: &'a [u8],
}

/// Decode width-byte, height-byte, palette-index pixels per BOB frame.
/// Palette application and rider remaps are deliberately separate.
/// Exact consumption is required; source-version adapters handle any future padding.
pub fn decode_bob(bytes: &[u8], frame_count: usize) -> Result<Vec<IndexedFrame<'_>>, DecodeError> {
    if frame_count > 4096 {
        return Err(DecodeError::TooManyFrames);
    }
    let mut offset = 0usize;
    let mut frames = Vec::with_capacity(frame_count);
    for _ in 0..frame_count {
        let header = bytes
            .get(offset..offset + 2)
            .ok_or(DecodeError::Truncated { offset, length: 2 })?;
        let (width, height) = (header[0], header[1]);
        if width == 0 || height == 0 {
            return Err(DecodeError::InvalidDimensions);
        }
        offset += 2;
        let length = usize::from(width) * usize::from(height);
        let pixels = bytes
            .get(offset..offset + length)
            .ok_or(DecodeError::Truncated { offset, length })?;
        frames.push(IndexedFrame {
            width,
            height,
            pixels,
        });
        offset += length;
    }
    if offset != bytes.len() {
        return Err(DecodeError::UnexpectedTrailingData);
    }
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_multiple_authored_frames_and_retains_indices() {
        let bytes = [2, 1, 0, 255, 1, 2, 7, 8];
        let frames = decode_bob(&bytes, 2).unwrap();
        assert_eq!(
            frames[0],
            IndexedFrame {
                width: 2,
                height: 1,
                pixels: &[0, 255]
            }
        );
        assert_eq!(frames[1].pixels, &[7, 8]);
    }
    #[test]
    fn rejects_every_truncation_of_a_frame() {
        let bytes = [2, 2, 0, 1, 2, 3];
        for length in 0..bytes.len() {
            assert!(decode_bob(&bytes[..length], 1).is_err());
        }
    }
    #[test]
    fn rejects_zero_dimensions_trailing_bytes_and_excessive_count() {
        assert_eq!(decode_bob(&[0, 1], 1), Err(DecodeError::InvalidDimensions));
        assert_eq!(
            decode_bob(&[1, 1, 4, 8], 1),
            Err(DecodeError::UnexpectedTrailingData)
        );
        assert_eq!(decode_bob(&[], 4097), Err(DecodeError::TooManyFrames));
        assert_eq!(decode_bob(&[], 0), Ok(vec![]));
    }
}
