use rspotify::model::track::FullTrack;
use rspotify::model::Id;
use rspotify::model::SimplifiedArtist;
use std::env;
use std::io::Read;
use std::io::Write;
mod auth;
mod connect;

mod downloader;
use std::process::{Command, Stdio};

#[tokio::main]
async fn main() {
    let client_id = env::var("WEB_CLIENT_ID").unwrap();
    let client_secret = env::var("WEB_CLIENT_SECRET").unwrap();
    let auth = auth::Auth::new(&client_id, &client_secret).await;
    let token = auth.get_access_token().await;

    let connect = connect::Connect::new(&client_id, &client_secret).await;

    let results = loop {
        let query = read_human::read_string_nonempty("Search Query").unwrap();
        let results = connect.search(&query).await;
        if results.len() > 0 {
            break results;
        } else {
            eprintln!("No Results Found");
        }
    };

    for (index, track) in results.iter().enumerate().rev() {
        let name = &track.name;
        let artists = (&track.artists)
            .into_iter()
            .map(|artist| artist.name.as_ref())
            .collect::<Vec<&str>>()
            .join(", ");
        println!(
            "{}. \u{001b}[31m{}\u{001b}[0m by \u{001b}[36m{}\u{001b}[0m",
            index, name, artists
        );
    }

    let choice = loop {
        let result: usize =
            read_human::read_custom_nonempty("What track do you want to modify").unwrap();
        if result <= results.len() {
            break result;
        }
    };

    let track = &results[choice];

    let speed = loop {
        let result: f32 = read_human::read_custom_nonempty("Playback Speed").unwrap();
        if result >= 0.5 && result <= 100.0 {
            break result;
        } else {
            eprintln!("Value out of range 0.5-100.0");
        }
    };

    let mut downloader = downloader::Downloader::new(token).await;
    println!("Downloading song");
    let ogg = downloader
        .get_ogg(&track.id.as_ref().unwrap().uri())
        .await
        .unwrap();
    println!("Done downloading");
    let metadata = get_metadata(track).await;
    let mp3 = convert_to_mp3(ogg, speed);
    let fiile_path = dirs::home_dir()
        .unwrap()
        .join("music")
        .join(get_mp3_filename(&track));
    let mut file = std::fs::File::create(fiile_path).unwrap();
    file.write_all(&metadata).unwrap();
    file.write_all(&mp3).unwrap();
    println!("Done");
}

fn convert_to_mp3(input: Vec<u8>, speed: f32) -> Vec<u8> {
    let mut ffmpeg_command = Command::new("ffmpeg")
        .args(&[
            "-hide_banner",
            "-i",
            "-",
            "-filter:a",
            &format!("atempo={}", speed),
            "-f",
            "mp3",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = ffmpeg_command.stdin.take().unwrap();
    std::thread::spawn(move || {
        stdin.write_all(&input).unwrap();
    });
    let mut buf = Vec::new();
    let mut stdout = ffmpeg_command.stdout.take().unwrap();
    stdout.read_to_end(&mut buf).unwrap();
    ffmpeg_command.wait().unwrap();
    buf
}

fn get_mp3_filename(track: &FullTrack) -> String {
    let ext = ".mp3";
    let ext_length = ext.as_bytes().len();
    let title = track.name.replace("/", "");
    let artist = stringify_artists(&track.artists).replace("/", "");
    let mut result = title + " - " + &artist;
    while result.as_bytes().len() > (255 - ext_length) {
        result.pop();
    }
    result + ext
}

fn stringify_artists(artists: &Vec<SimplifiedArtist>)-> String {
    artists.into_iter()
    .map(|artist| artist.name.as_ref())
    .collect::<Vec<&str>>()
    .join(", ")
}

async fn get_metadata(track: &FullTrack) -> Vec<u8> {
    let mut tag = id3::Tag::new();
    tag.set_title(&track.name);
    tag.set_artist(stringify_artists(&track.artists));
    tag.set_album(&track.album.name);
    let images = &track.album.images;
    if images.len() > 0 {
        println!("downloading cover art");
        let res = reqwest::get(&images[0].url).await;
        match res {
            Ok(response) => {
                if response.status() == 200 {
                    let mime = response
                        .headers()
                        .get("Content-Type")
                        .map_or("image/jpeg", |x| x.to_str().unwrap())
                        .to_owned();
                    let image_data = response.bytes().await.unwrap().to_vec();
                    tag.add_picture(id3::frame::Picture {
                        mime_type: mime,
                        description: "Cover".to_string(),
                        picture_type: id3::frame::PictureType::CoverFront,
                        data: image_data,
                    });
                } else {
                    println!(
                        "Got response code {} when retreiving cover art. Response text {}",
                        response.status(),
                        response.text().await.expect("Can't get response body?")
                    );
                }
            }
            Err(e) => println!("Error retreiving cover art: {}", e),
        }
    }

    let mut buf: Vec<u8> = vec![];
    tag.write_to(&mut buf, id3::Version::Id3v24).unwrap();
    buf
}
