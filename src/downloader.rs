extern crate env_logger;
extern crate librespot;
extern crate log;

use futures::channel::oneshot;
use librespot::audio::{AudioDecrypt, AudioFile};
use librespot::core::authentication::Credentials;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::core::spotify_id::SpotifyId;
use librespot::metadata::audio::{AudioFileFormat, AudioItem};
use std::borrow::Cow;
use std::io::Read;
use std::io::Seek;

pub struct Downloader {
    session: Session,
}

impl Downloader {
    pub async fn new(access_token: String) -> Downloader {
        let session = Session::new(SessionConfig::default(), None);
        let credentials = Credentials::with_access_token(access_token);
        session.connect(credentials, false).await.unwrap();
        Downloader {
            session: session,
        }
    }

    pub async fn get_ogg(&mut self, track_uri: &str) -> Option<Vec<u8>> {
        let track_id = SpotifyId::from_uri(track_uri).unwrap();

        let audio = AudioItem::get_file(&self.session, track_id).await.unwrap();

        let audio = match find_available_alternative(&self.session, &audio).await {
            Some(audio) => audio,
            None => {
                println!("<{}> is not available", audio.uri);
                return None;
            }
        };

        let formats = [
            AudioFileFormat::OGG_VORBIS_320,
            AudioFileFormat::OGG_VORBIS_160,
            AudioFileFormat::OGG_VORBIS_96,
        ];
        let format = formats
            .iter()
            .find(|format| audio.files.contains_key(format))
            .unwrap();

        let file_id = match audio.files.get(&format) {
            Some(&file_id) => file_id,
            None => {
                println!("<{}> in not available in format {:?}", audio.name, format);
                return None;
            }
        };

        let bytes_per_second = stream_data_rate(*format);

        let encrypted_file = AudioFile::open(&self.session, file_id, bytes_per_second).await.unwrap();
        let stream_loader_controller = encrypted_file.get_stream_loader_controller().unwrap();
        stream_loader_controller.set_stream_mode();
        let key = self.session.audio_key().request(track_id, file_id).await.unwrap();
        let mut decrypted_file = AudioDecrypt::new(Some(key), encrypted_file);
        let (tx, rx) = oneshot::channel();
        std::thread::spawn(move || {
            decrypted_file.seek(std::io::SeekFrom::Start(0xA7)).unwrap();
            let mut result = Vec::new();
            decrypted_file.read_to_end(&mut result).unwrap();
            tx.send(result).unwrap();
        });
        Some(rx.await.unwrap())
    }
}


async fn find_available_alternative<'a>(
    session: &Session,
    audio: &'a AudioItem,
) -> Option<Cow<'a, AudioItem>> {
    if audio.availability.is_ok() {
        Some(Cow::Borrowed(audio))
    } else {
        if let Some(alternatives) = &audio.alternatives {
            for alt_id in alternatives.iter(){
                let item = AudioItem::get_file(&session, alt_id.to_owned()).await.unwrap();
                if item.availability.is_ok(){
                    return Some(Cow::Owned(item));
                }
            }
            None
        } else {
            None
        }
    }
}

fn stream_data_rate(format: AudioFileFormat) -> usize {
    let kbps = match format {
        AudioFileFormat::OGG_VORBIS_96 => 12,
        AudioFileFormat::OGG_VORBIS_160 => 20,
        AudioFileFormat::OGG_VORBIS_320 => 40,
        AudioFileFormat::MP3_256 => 32,
        AudioFileFormat::MP3_320 => 40,
        AudioFileFormat::MP3_160 => 20,
        AudioFileFormat::MP3_96 => 12,
        AudioFileFormat::MP3_160_ENC => 20,
        AudioFileFormat::AAC_24 => 3,
        AudioFileFormat::AAC_48 => 6,
        AudioFileFormat::AAC_160 => 20,
        AudioFileFormat::AAC_320 => 40,
        AudioFileFormat::MP4_128 => 16,
        AudioFileFormat::OTHER5 => 40,
        AudioFileFormat::FLAC_FLAC => 112, // assume 900 kbit/s on average
        AudioFileFormat::UNKNOWN_FORMAT => todo!(),
    };
    kbps * 1024
}
