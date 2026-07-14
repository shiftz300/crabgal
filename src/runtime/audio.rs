#[cfg(feature = "audio-opus")]
use std::io::{self, Cursor};
#[cfg(feature = "audio-opus")]
use std::sync::Arc;
#[cfg(feature = "audio-opus")]
use std::time::Duration;

#[cfg(feature = "audio-opus")]
use bevy::asset::{AssetApp, AssetLoader, LoadContext, io::Reader};
#[cfg(feature = "audio-opus")]
use bevy::audio::{AddAudioSource, Decodable};
use bevy::audio::{AudioPlayer, AudioSource};
use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;
#[cfg(feature = "audio-opus")]
use rodio::{ChannelCount, SampleRate, Source};
#[cfg(feature = "audio-opus")]
use symphonia::core::audio::sample::Sample;
#[cfg(feature = "audio-opus")]
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
#[cfg(feature = "audio-opus")]
use symphonia::core::codecs::registry::CodecRegistry;
#[cfg(feature = "audio-opus")]
use symphonia::core::errors::Error as SymphoniaError;
#[cfg(feature = "audio-opus")]
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType, probe::Hint};
#[cfg(feature = "audio-opus")]
use symphonia::core::io::MediaSourceStream;
#[cfg(feature = "audio-opus")]
use symphonia::core::meta::MetadataOptions;
#[cfg(feature = "audio-opus")]
use symphonia_adapter_libopus::OpusDecoder;

/// Compressed Ogg Opus data. Decoding remains incremental during playback so
/// long voice and BGM assets never become full-size PCM allocations.
#[derive(Asset, Clone, Debug, TypePath)]
#[cfg(feature = "audio-opus")]
pub(crate) struct OpusAudio {
    bytes: Arc<[u8]>,
}

#[derive(Default, TypePath)]
#[cfg(feature = "audio-opus")]
struct OpusAudioLoader;

#[cfg(feature = "audio-opus")]
impl AssetLoader for OpusAudioLoader {
    type Asset = OpusAudio;
    type Settings = ();
    type Error = io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        if bytes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "empty Opus asset",
            ));
        }
        Ok(OpusAudio {
            bytes: bytes.into(),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["opus"]
    }
}

#[cfg(feature = "audio-opus")]
pub(crate) struct OpusAudioPlugin;

#[cfg(feature = "audio-opus")]
impl Plugin for OpusAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<OpusAudioLoader>()
            .add_audio_source::<OpusAudio>();
    }
}

#[cfg(feature = "audio-opus")]
impl Decodable for OpusAudio {
    type Decoder = OpusStream;

    fn decoder(&self) -> Self::Decoder {
        OpusStream::new(self.bytes.clone()).unwrap_or_else(|error| {
            log::error!("failed to decode Ogg Opus asset: {error}");
            OpusStream::failed()
        })
    }
}

/// Adds the correct Bevy audio player for a logical asset path. Projects can
/// keep using the same BGM/voice/effect commands while distribution switches
/// those files to `.opus`.
pub(crate) fn insert_player(
    entity: &mut EntityCommands<'_>,
    asset_server: &AssetServer,
    path: String,
) {
    if is_opus(&path) {
        #[cfg(feature = "audio-opus")]
        {
            entity.insert(AudioPlayer::<OpusAudio>(asset_server.load(path)));
            return;
        }
        #[cfg(not(feature = "audio-opus"))]
        log::error!("Opus asset `{path}` requires the `audio-opus` feature");
    }
    entity.insert(AudioPlayer::new(asset_server.load::<AudioSource>(path)));
}

pub(crate) fn load_untyped(asset_server: &AssetServer, path: String) -> UntypedHandle {
    if is_opus(&path) {
        #[cfg(feature = "audio-opus")]
        return asset_server.load::<OpusAudio>(path).untyped();
        #[cfg(not(feature = "audio-opus"))]
        log::error!("Opus asset `{path}` requires the `audio-opus` feature");
    }
    asset_server.load::<AudioSource>(path).untyped()
}

fn is_opus(path: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("opus"))
}

#[cfg(feature = "audio-opus")]
pub(crate) struct OpusStream {
    format: Option<Box<dyn FormatReader>>,
    decoder: Option<Box<dyn AudioDecoder>>,
    track_id: u32,
    samples: Vec<f32>,
    position: usize,
    channels: ChannelCount,
    sample_rate: SampleRate,
    ended: bool,
}

#[cfg(feature = "audio-opus")]
impl OpusStream {
    fn new(bytes: Arc<[u8]>) -> Result<Self, SymphoniaError> {
        let source = Box::new(Cursor::new(bytes));
        let stream = MediaSourceStream::new(source, Default::default());
        let mut hint = Hint::new();
        hint.with_extension("opus");
        let format_options = FormatOptions::default();
        let metadata_options = MetadataOptions::default();
        let format = symphonia::default::get_probe().probe(
            &hint,
            stream,
            format_options,
            metadata_options,
        )?;
        let track = format
            .default_track(TrackType::Audio)
            .ok_or(SymphoniaError::Unsupported("opus: no audio track"))?;
        let params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .ok_or(SymphoniaError::Unsupported("opus: invalid audio track"))?;
        let channels = ChannelCount::new(
            params
                .channels
                .as_ref()
                .map_or(2, |channels| channels.count() as u16),
        )
        .unwrap_or(ChannelCount::MIN);
        let sample_rate =
            SampleRate::new(params.sample_rate.unwrap_or(48_000)).unwrap_or(SampleRate::MIN);
        let track_id = track.id;

        let mut codecs = CodecRegistry::new();
        codecs.register_audio_decoder::<OpusDecoder>();
        let decoder = codecs.make_audio_decoder(params, &AudioDecoderOptions::default())?;

        Ok(Self {
            format: Some(format),
            decoder: Some(decoder),
            track_id,
            samples: Vec::new(),
            position: 0,
            channels,
            sample_rate,
            ended: false,
        })
    }

    fn failed() -> Self {
        Self {
            format: None,
            decoder: None,
            track_id: 0,
            samples: Vec::new(),
            position: 0,
            channels: ChannelCount::new(2).expect("stereo channel count is non-zero"),
            sample_rate: SampleRate::new(48_000).expect("Opus sample rate is non-zero"),
            ended: true,
        }
    }

    fn decode_next_packet(&mut self) -> bool {
        let (Some(format), Some(decoder)) = (&mut self.format, &mut self.decoder) else {
            self.ended = true;
            return false;
        };
        loop {
            let packet = match format.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => {
                    self.ended = true;
                    return false;
                }
                Err(SymphoniaError::ResetRequired) => {
                    decoder.reset();
                    continue;
                }
                Err(error) => {
                    if !matches!(error, SymphoniaError::IoError(_)) {
                        log::warn!("Ogg Opus packet read failed: {error}");
                    }
                    self.ended = true;
                    return false;
                }
            };
            if packet.track_id != self.track_id {
                continue;
            }
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    self.channels = ChannelCount::new(decoded.spec().channels().count() as u16)
                        .unwrap_or(ChannelCount::MIN);
                    self.sample_rate =
                        SampleRate::new(decoded.spec().rate()).unwrap_or(SampleRate::MIN);
                    self.samples.resize(decoded.samples_interleaved(), f32::MID);
                    decoded.copy_to_slice_interleaved(&mut self.samples);
                    self.position = 0;
                    if !self.samples.is_empty() {
                        return true;
                    }
                }
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(error) => {
                    log::warn!("Ogg Opus decode failed: {error}");
                    self.ended = true;
                    return false;
                }
            }
        }
    }
}

#[cfg(feature = "audio-opus")]
impl Iterator for OpusStream {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(sample) = self.samples.get(self.position).copied() {
                self.position += 1;
                return Some(sample);
            }
            if self.ended || !self.decode_next_packet() {
                return None;
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let buffered = self.samples.len().saturating_sub(self.position);
        (buffered, None)
    }
}

#[cfg(feature = "audio-opus")]
impl Source for OpusStream {
    fn current_span_len(&self) -> Option<usize> {
        self.ended.then_some(0)
    }

    fn channels(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(all(test, feature = "audio-opus"))]
mod tests {
    use super::*;

    #[test]
    fn decodes_project_ogg_opus_incrementally() {
        let bytes: Arc<[u8]> =
            include_bytes!("../../projects/test-project/content/shared/vocal/v16.opus")
                .as_slice()
                .into();
        let mut stream = OpusStream::new(bytes).expect("test Opus asset should open");
        let samples = stream.by_ref().take(4_800).collect::<Vec<_>>();

        assert_eq!(stream.channels().get(), 1);
        assert_eq!(stream.sample_rate().get(), 48_000);
        assert_eq!(samples.len(), 4_800);
        assert!(samples.iter().any(|sample| sample.abs() > f32::EPSILON));
    }

    #[test]
    fn decodes_embedded_webgal_k_ui_cues() {
        let cues: [(&[u8], f32); 3] = [
            (include_bytes!("../ui/assets/audio/click.opus"), 0.25),
            (include_bytes!("../ui/assets/audio/mouse-enter.opus"), 0.08),
            (include_bytes!("../ui/assets/audio/switch.opus"), 0.25),
        ];
        for (cue, minimum_seconds) in cues {
            let bytes: Arc<[u8]> = cue.into();
            let mut stream = OpusStream::new(bytes).expect("UI cue should open");
            assert_eq!(stream.channels().get(), 2);
            assert_eq!(stream.sample_rate().get(), 48_000);
            let channels = stream.channels().get() as usize;
            let sample_rate = stream.sample_rate().get() as usize;
            let samples = stream.by_ref().collect::<Vec<_>>();
            assert!(samples.iter().any(|sample| *sample != 0.0));
            let seconds = samples.len() as f32 / channels as f32 / sample_rate as f32;
            assert!(seconds >= minimum_seconds, "decoded only {seconds:.3}s");
        }
    }
}
