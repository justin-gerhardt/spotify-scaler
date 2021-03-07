extern crate env_logger;
extern crate librespot;
extern crate log;
extern crate tokio_core;
extern crate tokio_timer;

use compat::Compat;
use futures::channel::oneshot;
use futures::compat;
use librespot::audio::{AudioDecrypt, AudioFile};
use librespot::core::authentication::Credentials;
use librespot::core::config::SessionConfig;
use librespot::core::session::Session;
use librespot::core::spotify_id::SpotifyId;
use librespot::metadata::{AudioItem, FileFormat};
use std::borrow::Cow;
use std::io::Read;
use std::io::Seek;
use tokio_core::reactor::Core;

pub struct Downloader {
    core: Core,
    session: Session,
}

impl Downloader {
    pub fn new(username: String, password: String) -> Downloader {
        let mut core = Core::new().unwrap();
        let credentials = Credentials::with_password(username, password);
        println!("Connecting ..");
        let session = core
            .run(Session::connect(
                SessionConfig::default(),
                credentials,
                None,
                core.handle(),
            ))
            .unwrap();
        Downloader {
            core: core,
            session: session,
        }
    }

    pub fn get_ogg(&mut self, track_uri: &str) -> Option<Vec<u8>> {
        let track_id = SpotifyId::from_uri(track_uri).unwrap();
        let audio = self
            .core
            .run(librespot::metadata::AudioItem::get_audio_item(
                &self.session,
                track_id,
            ))
            .unwrap();

        let audio = match find_available_alternative(&self.session, &audio, &mut self.core) {
            Some(audio) => audio,
            None => {
                println!("<{}> is not available", audio.uri);
                return None;
            }
        };

        let formats = [
            FileFormat::OGG_VORBIS_320,
            FileFormat::OGG_VORBIS_160,
            FileFormat::OGG_VORBIS_96,
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

        let key = self.session.audio_key().request(track_id, file_id);
        let encrypted_file = AudioFile::open(&self.session, file_id, bytes_per_second, true);
        let encrypted_file = self.core.run(encrypted_file).unwrap();
        let mut stream_loader_controller = encrypted_file.get_stream_loader_controller();
        stream_loader_controller.set_stream_mode();
        let key = self.core.run(key).unwrap();
        let mut decrypted_file = AudioDecrypt::new(key, encrypted_file);
        let (tx, rx) = oneshot::channel();
        std::thread::spawn(move || {
            decrypted_file.seek(std::io::SeekFrom::Start(0xA7)).unwrap();
            let mut result = Vec::new();
            decrypted_file.read_to_end(&mut result).unwrap();
            tx.send(result).unwrap();
        });
        Some(self.core.run(Compat::new(rx)).unwrap())
    }
}
fn find_available_alternative<'a>(
    session: &Session,
    audio: &'a AudioItem,
    core: &mut Core,
) -> Option<Cow<'a, AudioItem>> {
    if audio.available {
        Some(Cow::Borrowed(audio))
    } else {
        if let Some(alternatives) = &audio.alternatives {
            let alternatives = alternatives.iter().map(|alt_id| {
                core.run(AudioItem::get_audio_item(session, *alt_id))
                    .unwrap()
            });
            alternatives
                .into_iter()
                .find(|alt| alt.available)
                .map(Cow::Owned)
        } else {
            None
        }
    }
}

fn stream_data_rate(format: FileFormat) -> usize {
    match format {
        FileFormat::OGG_VORBIS_96 => 12 * 1024,
        FileFormat::OGG_VORBIS_160 => 20 * 1024,
        FileFormat::OGG_VORBIS_320 => 40 * 1024,
        FileFormat::MP3_256 => 32 * 1024,
        FileFormat::MP3_320 => 40 * 1024,
        FileFormat::MP3_160 => 20 * 1024,
        FileFormat::MP3_96 => 12 * 1024,
        FileFormat::MP3_160_ENC => 20 * 1024,
        FileFormat::MP4_128_DUAL => 16 * 1024,
        FileFormat::OTHER3 => 40 * 1024, // better some high guess than nothing
        FileFormat::AAC_160 => 20 * 1024,
        FileFormat::AAC_320 => 40 * 1024,
        FileFormat::MP4_128 => 16 * 1024,
        FileFormat::OTHER5 => 40 * 1024, // better some high guess than nothing
    }
}
