extern crate rspotify;

use rspotify::client::Spotify;
use rspotify::model::track::FullTrack;
use rspotify::oauth2::SpotifyClientCredentials;
use rspotify::senum::Country;
use tokio::runtime::Runtime;

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
        Runtime::new().unwrap().block_on(creds.get_access_token());
        let spotify = Spotify::default().client_credentials_manager(creds);
        Connect { spotify: spotify }
    }

    pub fn search(&self, query: &str) -> Vec<FullTrack> {
        let future = self.spotify.search(
            query,
            rspotify::senum::SearchType::Track,
            25,
            0,
            Some(Country::Canada),
            None,
        );
        match Runtime::new().unwrap().block_on(future).unwrap() {
            rspotify::model::search::SearchResult::Tracks(x) => x.items,
            _ => {
                panic!("track search returned non-tracks");
            }
        }
        // self.spotify
        //     .search_track(query, 25, 0, Some(Country::Canada))
        //     .unwrap()
        //     .tracks
        //     .items
    }
}
