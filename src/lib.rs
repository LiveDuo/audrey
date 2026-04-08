
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum Format {
    #[cfg(feature = "flac")]
    Flac,
    #[cfg(feature = "ogg_vorbis")]
    OggVorbis,
    #[cfg(feature = "wav")]
    Wav,
}

impl Format {
    pub fn from_extension(extension: &str) -> Option<Self> {
        match extension {
            #[cfg(feature = "flac")]
            "flac" => Some(Format::Flac),
            #[cfg(feature = "ogg_vorbis")]
            "ogg" | "oga" => Some(Format::OggVorbis),
            #[cfg(feature = "wav")]
            "wav" | "wave" => Some(Format::Wav),
            #[cfg(feature = "caf")]
            "caf" => Some(Format::CafAlac),
            _ => None,
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            #[cfg(feature = "flac")]
            Format::Flac => "flac",
            #[cfg(feature = "wav")]
            Format::Wav => "wav",
            #[cfg(feature = "ogg_vorbis")]
            Format::OggVorbis => "ogg",
        }
    }
}


#[cfg(feature = "caf")]
use caf::{self, CafError};
#[cfg(feature = "flac")]
use claxon;
#[cfg(feature = "wav")]
use hound;
#[cfg(feature = "ogg_vorbis")]
use lewton;

pub trait Sample:
    dasp_sample::Sample
    + dasp_sample::FromSample<i8>
    + dasp_sample::FromSample<i16>
    + dasp_sample::FromSample<dasp_sample::I24>
    + dasp_sample::FromSample<i32>
    + dasp_sample::FromSample<f32>
{
}

impl<T> Sample for T where
    T: dasp_sample::Sample
        + dasp_sample::FromSample<i8>
        + dasp_sample::FromSample<i16>
        + dasp_sample::FromSample<dasp_sample::I24>
        + dasp_sample::FromSample<i32>
        + dasp_sample::FromSample<f32>
{
}

pub enum Reader<R>
where
    R: std::io::Read + std::io::Seek,
{
    #[cfg(feature = "flac")]
    Flac(claxon::FlacReader<R>),
    #[cfg(feature = "ogg_vorbis")]
    OggVorbis(lewton::inside_ogg::OggStreamReader<R>),
    #[cfg(feature = "wav")]
    Wav(hound::WavReader<R>),
}

pub struct Samples<'a, R, S>
where
    R: 'a + std::io::Read + std::io::Seek,
{
    format: FormatSamples<'a, R>,
    sample: std::marker::PhantomData<S>,
}

enum FormatSamples<'a, R>
where
    R: 'a + std::io::Read + std::io::Seek,
{
    #[cfg(feature = "flac")]
    Flac {
        sample_bits: u32,
        flac_samples: claxon::FlacSamples<&'a mut claxon::input::BufferedReader<R>>,
    },

    #[cfg(feature = "flac")]
    FlacUnsupportedSampleBits(u32),

    #[cfg(feature = "ogg_vorbis")]
    OggVorbis {
        reader: &'a mut lewton::inside_ogg::OggStreamReader<R>,
        index: usize,
        buffer: Vec<i16>,
    },

    #[cfg(feature = "wav")]
    Wav(WavSamples<'a, R>),

    #[cfg(feature = "wav")]
    WavUnsupportedSampleBits(u16),

}

#[cfg(feature = "wav")]
enum WavSamples<'a, R: 'a> {
    I8(hound::WavSamples<'a, R, i8>),
    I16(hound::WavSamples<'a, R, i16>),
    I24(hound::WavSamples<'a, R, i32>),
    I32(hound::WavSamples<'a, R, i32>),
    F32(hound::WavSamples<'a, R, f32>),
}

pub struct Frames<'a, R, F>
where
    R: 'a + std::io::Read + std::io::Seek,
    F: dasp_frame::Frame,
{
    samples: Samples<'a, R, F::Sample>,
    frame: std::marker::PhantomData<F>,
}

pub type BufFileReader = Reader<std::io::BufReader<std::fs::File>>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Description {
    format: Format,
    channel_count: u32,
    sample_rate: u32,
}

#[derive(Debug)]
pub enum ReadError {
    Io(std::io::Error),
    Reader(FormatError),
    UnsupportedFormat,
}

#[derive(Debug)]
pub enum FormatError {
    #[cfg(feature = "flac")]
    Flac(claxon::Error),
    #[cfg(feature = "flac")]
    FlacUnsupportedSampleBits(u32),
    #[cfg(feature = "ogg_vorbis")]
    OggVorbis(lewton::VorbisError),
    #[cfg(feature = "wav")]
    Wav(hound::Error),
    #[cfg(feature = "wav")]
    WavUnsupportedSampleBits(u16),
    #[cfg(feature = "caf")]
    Caf(caf::CafError),
    #[cfg(feature = "alac")]
    Alac(()),
}

pub fn open<P>(file_path: P) -> Result<BufFileReader, ReadError>
where
    P: AsRef<std::path::Path>,
{
    BufFileReader::open(file_path)
}

impl Description {
    pub fn format(&self) -> Format {
        self.format
    }

    pub fn channel_count(&self) -> u32 {
        self.channel_count
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl BufFileReader {
    pub fn open<P>(file_path: P) -> Result<Self, ReadError>
    where
        P: AsRef<std::path::Path>,
    {
        let path = file_path.as_ref();
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        Reader::new(reader)
    }
}

impl<R> Reader<R>
where
    R: std::io::Read + std::io::Seek,
{
    pub fn new(mut reader: R) -> Result<Self, ReadError> {
        #[cfg(feature = "wav")]
        {
            let is_wav = match hound::WavReader::new(&mut reader) {
                Err(hound::Error::FormatError(_)) => false,
                Err(err) => return Err(err.into()),
                Ok(_) => true,
            };
            reader.seek(std::io::SeekFrom::Start(0))?;
            if is_wav {
                return Ok(Reader::Wav(hound::WavReader::new(reader)?));
            }
        }

        #[cfg(feature = "flac")]
        {
            let is_flac = match claxon::FlacReader::new(&mut reader) {
                Err(claxon::Error::FormatError(_)) => false,
                Err(err) => return Err(err.into()),
                Ok(_) => true,
            };
            reader.seek(std::io::SeekFrom::Start(0))?;
            if is_flac {
                return Ok(Reader::Flac(claxon::FlacReader::new(reader)?));
            }
        }

        #[cfg(feature = "ogg_vorbis")]
        {
            let is_ogg_vorbis = match lewton::inside_ogg::OggStreamReader::new(&mut reader) {
                Err(lewton::VorbisError::OggError(_))
                | Err(lewton::VorbisError::BadHeader(
                    lewton::header::HeaderReadError::NotVorbisHeader,
                )) => false,
                Err(err) => return Err(err.into()),
                Ok(_) => true,
            };
            reader.seek(std::io::SeekFrom::Start(0))?;
            if is_ogg_vorbis {
                return Ok(Reader::OggVorbis(lewton::inside_ogg::OggStreamReader::new(
                    reader,
                )?));
            }
        }

        Err(ReadError::UnsupportedFormat)
    }

    pub fn format(&self) -> Format {
        match *self {
            #[cfg(feature = "flac")]
            Reader::Flac(_) => Format::Flac,
            #[cfg(feature = "ogg_vorbis")]
            Reader::OggVorbis(_) => Format::OggVorbis,
            #[cfg(feature = "wav")]
            Reader::Wav(_) => Format::Wav,
        }
    }

    pub fn description(&self) -> Description {
        match *self {
            #[cfg(feature = "flac")]
            Reader::Flac(ref reader) => {
                let info = reader.streaminfo();
                Description {
                    format: Format::Flac,
                    channel_count: info.channels as u32,
                    sample_rate: info.sample_rate,
                }
            }

            #[cfg(feature = "ogg_vorbis")]
            Reader::OggVorbis(ref reader) => Description {
                format: Format::OggVorbis,
                channel_count: u32::from(reader.ident_hdr.audio_channels),
                sample_rate: reader.ident_hdr.audio_sample_rate as u32,
            },

            #[cfg(feature = "wav")]
            Reader::Wav(ref reader) => {
                let spec = reader.spec();
                Description {
                    format: Format::Wav,
                    channel_count: u32::from(spec.channels),
                    sample_rate: spec.sample_rate,
                }
            }
        }
    }

    pub fn samples<S>(&mut self) -> Samples<'_, R, S>
    where
        S: Sample,
    {
        let format = match *self {
            #[cfg(feature = "flac")]
            Reader::Flac(ref mut reader) => {
                let sample_bits = reader.streaminfo().bits_per_sample;
                if sample_bits > 32 {
                    FormatSamples::FlacUnsupportedSampleBits(sample_bits)
                } else {
                    FormatSamples::Flac {
                        sample_bits,
                        flac_samples: reader.samples(),
                    }
                }
            }

            #[cfg(feature = "ogg_vorbis")]
            Reader::OggVorbis(ref mut reader) => FormatSamples::OggVorbis {
                reader,
                index: 0,
                buffer: Vec::new(),
            },

            #[cfg(feature = "wav")]
            Reader::Wav(ref mut reader) => {
                let spec = reader.spec();
                match spec.sample_format {
                    hound::SampleFormat::Int => match spec.bits_per_sample {
                        8 => FormatSamples::Wav(WavSamples::I8(reader.samples())),
                        16 => FormatSamples::Wav(WavSamples::I16(reader.samples())),
                        24 => FormatSamples::Wav(WavSamples::I24(reader.samples())),
                        32 => FormatSamples::Wav(WavSamples::I32(reader.samples())),
                        _ => FormatSamples::WavUnsupportedSampleBits(spec.bits_per_sample),
                    },
                    hound::SampleFormat::Float => {
                        FormatSamples::Wav(WavSamples::F32(reader.samples()))
                    }
                }
            }
        };

        Samples {
            format,
            sample: std::marker::PhantomData,
        }
    }

    pub fn frames<F>(&mut self) -> Frames<'_, R, F>
    where
        F: dasp_frame::Frame,
        F::Sample: Sample,
    {
        Frames {
            samples: self.samples(),
            frame: std::marker::PhantomData,
        }
    }
}

impl<'a, R, S> Iterator for Samples<'a, R, S>
where
    R: std::io::Read + std::io::Seek,
    S: Sample,
{
    type Item = Result<S, FormatError>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.format {
            #[cfg(feature = "flac")]
            FormatSamples::Flac {
                sample_bits,
                ref mut flac_samples,
            } => flac_samples.next().map(|sample| {
                sample
                    .map_err(FormatError::Flac)
                    .map(|sample| sample << (32 - sample_bits))
                    .map(dasp_sample::Sample::to_sample)
            }),

            #[cfg(feature = "flac")]
            FormatSamples::FlacUnsupportedSampleBits(sample_bits) => {
                Some(Err(FormatError::FlacUnsupportedSampleBits(sample_bits)))
            }

            #[cfg(feature = "ogg_vorbis")]
            FormatSamples::OggVorbis {
                ref mut reader,
                ref mut index,
                ref mut buffer,
            } => loop {
                if *index < buffer.len() {
                    let sample = dasp_sample::Sample::to_sample(buffer[*index]);
                    *index += 1;
                    return Some(Ok(sample));
                }

                match reader.read_dec_packet_itl() {
                    Ok(Some(packet)) => {
                        let _ = std::mem::replace(buffer, packet);
                        *index = 0;
                    }
                    Ok(None) => return None,
                    Err(err) => return Some(Err(err.into())),
                }
            },

            #[cfg(feature = "wav")]
            FormatSamples::Wav(ref mut wav_samples) => {
                macro_rules! next_sample {
                    ($samples:expr) => {{
                        $samples.next().map(|sample| {
                            sample
                                .map_err(FormatError::Wav)
                                .map(dasp_sample::Sample::to_sample)
                        })
                    }};
                }

                match *wav_samples {
                    WavSamples::I8(ref mut samples) => next_sample!(samples),
                    WavSamples::I16(ref mut samples) => next_sample!(samples),
                    WavSamples::I24(ref mut samples) => samples.next().map(|sample| {
                        sample
                            .map_err(FormatError::Wav)
                            .map(dasp_sample::I24::new_unchecked)
                            .map(dasp_sample::Sample::to_sample)
                    }),
                    WavSamples::I32(ref mut samples) => next_sample!(samples),
                    WavSamples::F32(ref mut samples) => next_sample!(samples),
                }
            }

            #[cfg(feature = "wav")]
            FormatSamples::WavUnsupportedSampleBits(sample_bits) => {
                Some(Err(FormatError::WavUnsupportedSampleBits(sample_bits)))
            }

        }
    }
}

impl<'a, R, F> Iterator for Frames<'a, R, F>
where
    R: std::io::Read + std::io::Seek,
    F: dasp_frame::Frame,
    F::Sample: Sample,
{
    type Item = Result<F, FormatError>;
    fn next(&mut self) -> Option<Self::Item> {
        enum FrameConstruction {
            NotEnoughSamples,
            Ok,
            Err(FormatError),
        }

        let mut result = FrameConstruction::Ok;
        let frame = F::from_fn(|_| match self.samples.next() {
            Some(Ok(sample)) => sample,
            Some(Err(error)) => {
                result = FrameConstruction::Err(error);
                <F::Sample as dasp_sample::Sample>::EQUILIBRIUM
            }
            None => {
                result = FrameConstruction::NotEnoughSamples;
                <F::Sample as dasp_sample::Sample>::EQUILIBRIUM
            }
        });

        match result {
            FrameConstruction::Ok => Some(Ok(frame)),
            FrameConstruction::Err(error) => Some(Err(error)),
            FrameConstruction::NotEnoughSamples => None,
        }
    }
}

#[cfg(feature = "flac")]
impl From<claxon::Error> for FormatError {
    fn from(err: claxon::Error) -> Self {
        FormatError::Flac(err)
    }
}

#[cfg(feature = "ogg_vorbis")]
impl From<lewton::VorbisError> for FormatError {
    fn from(err: lewton::VorbisError) -> Self {
        FormatError::OggVorbis(err)
    }
}

#[cfg(feature = "wav")]
impl From<hound::Error> for FormatError {
    fn from(err: hound::Error) -> Self {
        FormatError::Wav(err)
    }
}

#[cfg(feature = "caf")]
impl From<CafError> for FormatError {
    fn from(err: CafError) -> Self {
        FormatError::Caf(err)
    }
}

impl<T> From<T> for ReadError
where
    T: Into<FormatError>,
{
    fn from(err: T) -> Self {
        ReadError::Reader(err.into())
    }
}

impl From<std::io::Error> for ReadError {
    fn from(err: std::io::Error) -> Self {
        ReadError::Io(err)
    }
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "flac")]
            FormatError::Flac(err) => write!(f, "{err}"),

            #[cfg(feature = "flac")]
            FormatError::FlacUnsupportedSampleBits(_) => {
                write!(f, "More than 32 bits per sample are not supported for Flac")
            }

            #[cfg(feature = "ogg_vorbis")]
            FormatError::OggVorbis(err) => write!(f, "{err}"),

            #[cfg(feature = "wav")]
            FormatError::Wav(err) => write!(f, "{err}"),

            #[cfg(feature = "wav")]
            FormatError::WavUnsupportedSampleBits(_) => {
                write!(f, "Only 8, 16, 24, 32 bits supported for integer wave")
            }

            #[cfg(feature = "caf")]
            FormatError::Caf(err) => write!(f, "{err}"),

            #[cfg(feature = "alac")]
            FormatError::Alac(_) => write!(f, "Alac decode error"),
        }
    }
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match *self {
            ReadError::Io(ref err) => err.fmt(f),
            ReadError::Reader(ref err) => err.fmt(f),
            ReadError::UnsupportedFormat => write!(f, "{}", self.to_string()),
        }
    }
}


#[cfg(test)]
mod tests {
    #![cfg(all(feature = "flac", feature = "ogg_vorbis", feature = "wav"))]

    const FLAC: &'static str = "samples/sine_440hz_stereo.flac";
    const OGG_VORBIS: &'static str = "samples/sine_440hz_stereo.ogg";
    const WAV: &'static str = "samples/sine_440hz_stereo.wav";

    #[test]
    fn read() {
        let flac = std::io::BufReader::new(std::fs::File::open(FLAC).unwrap());
        match crate::Reader::new(flac).unwrap() {
            crate::Reader::Flac(_) => (),
            _ => panic!("Incorrect audio format"),
        }
        let wav = std::io::BufReader::new(std::fs::File::open(WAV).unwrap());
        match crate::Reader::new(wav).unwrap() {
            crate::Reader::Wav(_) => (),
            _ => panic!("Incorrect audio format"),
        }
        let ogg_vorbis = std::io::BufReader::new(std::fs::File::open(OGG_VORBIS).unwrap());
        match crate::Reader::new(ogg_vorbis).unwrap() {
            crate::Reader::OggVorbis(_) => (),
            _ => panic!("Incorrect audio format"),
        }
    }

    #[test]
    fn open() {
        match crate::open(FLAC).unwrap() {
            crate::Reader::Flac(_) => (),
            _ => panic!("Incorrect audio format"),
        }
        match crate::open(WAV).unwrap() {
            crate::Reader::Wav(_) => (),
            _ => panic!("Incorrect audio format"),
        }
        match crate::open(OGG_VORBIS).unwrap() {
            crate::Reader::OggVorbis(_) => (),
            _ => panic!("Incorrect audio format"),
        }
    }

    #[test]
    fn open_and_read_samples() {
        fn read_samples<P>(path: P) -> usize
        where
            P: AsRef<std::path::Path>,
        {
            let mut reader = crate::open(path).unwrap();
            reader.samples::<i16>().map(Result::unwrap).count()
        }

        let num_wav_samples = read_samples(WAV);
        assert_eq!(num_wav_samples, read_samples(FLAC));
        read_samples(OGG_VORBIS);
    }

}
