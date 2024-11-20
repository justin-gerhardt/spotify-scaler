extern crate rspotify;

use rspotify::model::track::FullTrack;


use rspotify::{
    model::{Country, Market, SearchType},
    prelude::*,
    ClientCredsSpotify, Credentials,
};


pub struct Connect {
    spotify: ClientCredsSpotify,
}

// NOTE: the web api client doesn't refresh creds. They are only valid for an hour
impl Connect {
    pub async fn new(client_id: &str, client_secret: &str) -> Connect {

        let creds = Credentials {
            id: client_id.to_owned(),
            secret: Some(client_secret.to_owned()),
        };
        let spotify = ClientCredsSpotify::new(creds);
        spotify.request_token().await.unwrap();
        Connect { spotify: spotify }
    }

    pub async fn search(&self, query: &str) -> Vec<FullTrack> {

        let future = self.spotify.search(query, SearchType::Track, Some(Market::Country(Country::Canada)), None, Some(25), None);
        match future.await.unwrap() {
            rspotify::model::search::SearchResult::Tracks(x) => x.items,
            _ => {
                panic!("track search returned non-tracks");
            }
        }

    }
}
