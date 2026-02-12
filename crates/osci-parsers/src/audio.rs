use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatReader;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;

/// Decoded audio data with per-channel sample buffers.
pub struct AudioData {
    /// Sample data indexed by channel: `samples[channel][sample_index]`.
    pub samples: Vec<Vec<f32>>,
    /// Sample rate in Hz (e.g. 44100).
    pub sample_rate: u32,
    /// Number of audio channels.
    pub num_channels: usize,
    /// Number of samples per channel.
    pub num_samples: usize,
}

/// Decode audio file bytes into interleaved sample buffers.
///
/// Supports any format that symphonia can probe (MP3, FLAC, WAV, OGG, etc.).
/// Returns an `AudioData` struct containing per-channel f32 samples.
pub fn parse_audio(data: &[u8]) -> Result<AudioData, String> {
    let cursor = std::io::Cursor::new(data.to_vec());
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
    let hint = Hint::new();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &Default::default(), &Default::default())
        .map_err(|e| format!("probe error: {e}"))?;

    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "no audio track found".to_string())?;
    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
    let num_channels = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(2);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("decoder error: {e}"))?;

    let mut all_samples: Vec<Vec<f32>> = vec![Vec::new(); num_channels];

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(format!("packet error: {e}")),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder
            .decode(&packet)
            .map_err(|e| format!("decode error: {e}"))?;
        let spec = *decoded.spec();
        let mut sample_buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);

        let samples = sample_buf.samples();
        let ch = spec.channels.count();
        for (i, &s) in samples.iter().enumerate() {
            all_samples[i % ch].push(s);
        }
    }

    let num_samples = all_samples.first().map(|c| c.len()).unwrap_or(0);

    Ok(AudioData {
        samples: all_samples,
        sample_rate,
        num_channels,
        num_samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_data_struct() {
        let audio = AudioData {
            samples: vec![vec![0.0; 100], vec![0.0; 100]],
            sample_rate: 44100,
            num_channels: 2,
            num_samples: 100,
        };
        assert_eq!(audio.sample_rate, 44100);
        assert_eq!(audio.num_channels, 2);
        assert_eq!(audio.num_samples, 100);
        assert_eq!(audio.samples.len(), 2);
        assert_eq!(audio.samples[0].len(), 100);
    }

    #[test]
    fn test_invalid_audio_returns_error() {
        let result = parse_audio(b"not audio data");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_audio_returns_error() {
        let result = parse_audio(&[]);
        assert!(result.is_err());
    }
}
