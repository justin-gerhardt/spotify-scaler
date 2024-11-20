use std::{process::Command, thread};

use rspotify::{prelude::*, scopes, AuthCodeSpotify, Credentials, OAuth, Token};
use tiny_http::{Header, Response, Server};

use personal_kv::kv::KVStore;

pub(crate) struct Auth {
    client: AuthCodeSpotify,
    kv: Box<dyn KVStore>,
}

impl Auth {
    pub(crate) async fn new(client_id: &str, client_secret: &str) -> Self {
        let creds = Credentials {
            id: client_id.to_owned(),
            secret: Some(client_secret.to_owned()),
        };

        let oauth = OAuth {
            redirect_uri: "http://localhost:4816/callback".to_string(),
            scopes: scopes!("streaming"),
            ..Default::default()
        };
        Auth {
            client: AuthCodeSpotify::new(creds, oauth),
            kv: personal_kv::new(false).await.unwrap(),
        }
    }

    async fn do_fresh_auth(&self) {
        let url = self.client.get_authorize_url(false).unwrap();
        let child = thread::spawn(|| {
            let server = Server::http("localhost:4816").unwrap();
            let request = server.recv().unwrap();
            let callback = "http://localhost:4816".to_owned() + request.url();
            let response = Response::from_string(include_str!("callback.html"));
            let header: Header = "Content-Type: text/html; charset=utf-8".parse().unwrap();
            request.respond(response.with_header(header)).unwrap();
            callback
        });
        Command::new("xdg-open").arg(url).output().unwrap();
        let callback = child.join().unwrap();
        let code = self.client.parse_response_code(&callback).unwrap();
        self.client.request_token(&code).await.unwrap();
    }

    pub(crate) async fn get_access_token(&self) -> String {
        let refresh_token = self.kv.get("spotify_refresh_token").await.unwrap();
        if let Some(refresh_token) = refresh_token {
            let token = Token {
                refresh_token: Some(refresh_token),
                ..Default::default()
            };
            let mut client_token = self.client.token.lock().await.unwrap();
            *client_token = Some(token);
            drop(client_token);
            self.client.refresh_token().await.unwrap();
        } else {
            self.do_fresh_auth().await;
        }
        let new_token = self.client.token.lock().await.unwrap().clone().unwrap();
        let new_refresh = new_token.refresh_token.unwrap();
        self.kv
            .put("spotify_refresh_token", &new_refresh)
            .await
            .unwrap();
        new_token.access_token
    }
}
