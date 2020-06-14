extern crate rspotify;

use rspotify::spotify::client::Spotify;
use rspotify::spotify::model::track::FullTrack;
use rspotify::spotify::oauth2::SpotifyClientCredentials;
use rspotify::spotify::senum::Country;
pub struct Connect {
    spotify: Spotify,
}

// NOTE: the web api client doesn't refresh creds. They are only valid for an hour
impl Connect {
    pub fn new(client_id: &str, client_secret: &str) -> Connect {
        let creds = SpotifyClientCredentials::default()
            .client_id(client_id)
            .client_secret(client_secret)
            .build();
        let spotify = Spotify::default().client_credentials_manager(creds);
        Connect { spotify: spotify }
    }

    pub fn search(&self, query: &str) -> Vec<FullTrack> {
        self.spotify
            .search_track(query, 25, 0, Some(Country::Canada))
            .unwrap()
            .tracks
            .items
    }
}
