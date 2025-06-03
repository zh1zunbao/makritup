use std::io::Cursor;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::get_probe;
use hound::{WavSpec, WavWriter};

#[derive(Debug)]
pub enum AudioConversionError {
    UnsupportedFormat,
    DecodingError(String),
    EncodingError(String),
    IoError(std::io::Error),
}

impl From<std::io::Error> for AudioConversionError {
    fn from(err: std::io::Error) -> Self {
        AudioConversionError::IoError(err)
    }
}

impl From<SymphoniaError> for AudioConversionError {
    fn from(err: SymphoniaError) -> Self {
        AudioConversionError::DecodingError(err.to_string())
    }
}

pub fn audio_to_wav(input_bytes: &[u8]) -> Result<Vec<u8>, AudioConversionError> {
    // Create a cursor from input bytes (clone to owned Vec to satisfy lifetime requirements)
    let owned_bytes = input_bytes.to_vec();
    let cursor = Cursor::new(owned_bytes);
    let media_source = MediaSourceStream::new(Box::new(cursor), Default::default());

    // Create a probe hint (let symphonia auto-detect format)
    let hint = Hint::new();
    
    // Get the format reader
    let probed = get_probe()
        .format(&hint, media_source, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|_| AudioConversionError::UnsupportedFormat)?;

    let mut format_reader = probed.format;

    // Get the default track
    let track = format_reader
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(AudioConversionError::UnsupportedFormat)?;

    // Create decoder
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|_| AudioConversionError::UnsupportedFormat)?;

    let track_id = track.id;
    let mut sample_buffer = None;
    let mut samples: Vec<f32> = Vec::new();
    let mut sample_rate = 44100u32;
    let mut channels = 1u16;

    // Decode all packets
    loop {
        let packet = match format_reader.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(err) => return Err(AudioConversionError::DecodingError(err.to_string())),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder.decode(&packet)?;
        
        // Initialize sample buffer on first decode
        if sample_buffer.is_none() {
            let spec = *decoded.spec();
            sample_rate = spec.rate;
            channels = spec.channels.count() as u16;
            
            let capacity = decoded.capacity() as u64;
            sample_buffer = Some(SampleBuffer::<f32>::new(capacity, spec));
        }

        if let Some(ref mut buf) = sample_buffer {
            buf.copy_interleaved_ref(decoded);
            samples.extend_from_slice(buf.samples());
        }
    }

    // Convert to mono if needed
    let mono_samples = if channels > 1 {
        convert_to_mono(&samples, channels as usize)
    } else {
        samples
    };

    // Convert f32 samples to i16 and write to WAV
    let wav_data = create_wav_bytes(&mono_samples, sample_rate)?;

    Ok(wav_data)
}

fn convert_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }

    samples
        .chunks_exact(channels)
        .map(|frame| {
            // Mix all channels to mono by averaging
            frame.iter().sum::<f32>() / channels as f32
        })
        .collect()
}

fn create_wav_bytes(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>, AudioConversionError> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut wav_data = Vec::new();
    {
        let cursor = Cursor::new(&mut wav_data);
        let mut writer = WavWriter::new(cursor, spec)
            .map_err(|e| AudioConversionError::EncodingError(e.to_string()))?;

        // Convert f32 samples to i16
        for &sample in samples {
            let sample_i16 = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(sample_i16)
                .map_err(|e| AudioConversionError::EncodingError(e.to_string()))?;
        }

        writer.finalize()
            .map_err(|e| AudioConversionError::EncodingError(e.to_string()))?;
    }

    Ok(wav_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mono_conversion() {
        let stereo_samples = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
        let mono_samples = convert_to_mono(&stereo_samples, 2);
        
        assert_eq!(mono_samples.len(), 3);
        assert_eq!(mono_samples[0], 0.15); // (0.1 + 0.2) / 2
        assert_eq!(mono_samples[1], 0.35); // (0.3 + 0.4) / 2
        assert_eq!(mono_samples[2], 0.55); // (0.5 + 0.6) / 2
    }
}
