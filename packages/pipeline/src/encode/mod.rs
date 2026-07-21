mod encoder;
mod params;

use super::*;

pub use encoder::Encoder;
pub use params::EncodeParams;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_audio() {
        initialize(log::Level::Error).unwrap();

        let buffer = crate::tests::encoded_ogg();
        assert!(buffer.starts_with(b"OggS"));
    }
}
