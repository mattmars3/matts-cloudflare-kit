use chrono::{DateTime, TimeDelta};
use serde::{Deserialize, Serialize};
use serde_json::json;
use worker::{kv::KvStore, Router, *};

use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::model::{TimeRange, Token};
use rspotify::{AuthCodeSpotify, Credentials, OAuth};

use futures_util::StreamExt;

const REFRESH_INTERVAL_HOURS: i64 = 24 * 7;
const REFRESH_INTERVAL_SECONDS: i64 = REFRESH_INTERVAL_HOURS * 60 * 60;
const NUM_TOP_TRACKS_TO_COLLECT: usize = 4;
const NUM_TOP_TRACKS_TO_DISPLAY: usize = 3;

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<worker::Response> {
    console_error_panic_hook::set_once();

    Router::with_data(env.clone())
        .get_async("/api/top-tracks", |_, ctx| async move {
            console_log!("top-tracks route has been called");
            let Ok(kv_handle) = ctx.kv("SPOTIFY") else {
                console_error!("Unable to obtain a key-value handle");
                return Response::error("Server Error", 500);
            };
            let tracks_expired = tracks_are_expired(&kv_handle).await;

            if tracks_expired {
                console_log!("Tracks are expired.");
                let env_clone = ctx.data.clone();
                let _track_refresh_result = refresh_tracks(env_clone, &kv_handle).await;
                set_last_refresh_time(&kv_handle).await;
                return Response::ok(generate_htmx(
                    json!(_track_refresh_result.expect("Unable to unwrap newly refreshed tracks"))
                        .to_string(),
                ));
            } else {
                console_log!("Returning cached tracks");
                return Response::ok(generate_htmx(get_cached_tracks(&kv_handle).await));
            }
        })
        .run(req, env)
        .await
}

/// Pulls the last time tracks were fetched from KV storage. Return whether or not the tracks are
/// expired - determined by a constant
pub async fn tracks_are_expired(kv_handle: &KvStore) -> bool {
    let last_refresh_time_option = get_elapsed_since_last_refresh_time(kv_handle).await;

    if let Some(last_refresh_time) = last_refresh_time_option {
        console_debug!("Found last refresh time");
        if last_refresh_time > REFRESH_INTERVAL_SECONDS {
            true
        } else {
            false
        }
    } else {
        console_error!("Could not find last refresh time");
        // not found, refresh anyway
        true
    }
}

/// Returns the last time spotify top items were fetch in unix time
pub async fn get_elapsed_since_last_refresh_time(kv_handle: &KvStore) -> Option<i64> {
    if let Ok(Some(raw)) = kv_handle.get("last_refresh_time").text().await {
        let timestamp: i64 = raw.parse().unwrap_or(0);
        let now_seconds = (js_sys::Date::now() / 1000.0) as i64;

        let elapsed = now_seconds - timestamp;
        Some(elapsed)
    } else {
        None
    }
}

pub async fn refresh_tracks(
    env: worker::Env,
    kv_handle: &KvStore,
) -> std::result::Result<Vec<DisplayTopTrack>, worker::Error> {
    let spotify_handle = get_spotify_api_handle(&env).await?;

    let mut top_tracks_stream = spotify_handle.current_user_top_tracks(Some(TimeRange::ShortTerm));

    let mut top_tracks_collection: Vec<DisplayTopTrack> = vec![];

    let mut num_top_tracks_collected: usize = 0;
    while let Some(top_item) = top_tracks_stream.next().await {
        // exit early if track collection limit reached
        if num_top_tracks_collected >= NUM_TOP_TRACKS_TO_COLLECT {
            break;
        }

        match top_item {
            Ok(track) => top_tracks_collection.push(DisplayTopTrack {
                track_name: track.name.clone(),
                track_name_link: track
                    .external_urls
                    .get("spotify")
                    .unwrap_or(&"".to_string())
                    .to_string(),
                track_artist: track.artists[0].name.clone(),
                track_artist_link: track.artists[0]
                    .external_urls
                    .get("spotify")
                    .unwrap_or(&"".to_string())
                    .to_string(),
                track_album_art_url: track.album.images[0].url.clone(),
            }),
            Err(_e) => {
                return Err(worker::Error::RouteNoDataError); // NOTE: fix this since the error
                                                             // description is insufficient for representing the fact that no data could be
                                                             // fetched
            }
        }

        num_top_tracks_collected += 1;
    }

    let stringified_top_tracks: String = json!(top_tracks_collection).to_string();

    console_log!("Storing Spotify top tracks");
    // now that you have collected the top artists, push them into KV Storage
    let _kv_result = kv_handle
        .put("SPOTIFY_TOP_TRACKS", stringified_top_tracks)
        .map_err(|e| worker::Error::RustError(format!("KV put failed: {e}")))?
        .execute()
        .await
        .map_err(|e| worker::Error::RustError(format!("KV execute failed: {e}")))?;

    console_log!("Successfully stored spotify top tracks in KV storage");

    Ok(top_tracks_collection)
}

/// Sets the last_refresh_time key to current time.
pub async fn set_last_refresh_time(kv_handle: &KvStore) {
    let kv_result = kv_handle
        .put(
            "last_refresh_time",
            chrono::Utc::now().timestamp().to_string(),
        )
        .expect("Unable to set last refresh time")
        .execute()
        .await;
    if kv_result.is_ok() {
        console_log!("Successfully set last refresh time");
    } else {
        console_error!("Failed to set last refresh time");
    }
}

/// Pulls cached response from KV-storage and returns it. Does no checking to see if expired.
pub async fn get_cached_tracks(kv_handle: &KvStore) -> String {
    if let Ok(Some(top_tracks_text)) = kv_handle.get("SPOTIFY_TOP_TRACKS").text().await {
        console_log!("Returning cached tracks.");
        top_tracks_text
    } else {
        console_error!("No cached tracks found in key-value storage");
        "{}".to_string()
    }
}

#[derive(Debug)]
pub struct SpotifySecrets {
    pub spotify_client_id: String,
    pub spotify_client_secret: String,
    pub spotify_refresh_token: String,
    pub spotify_redirect_uri: String,
}

impl SpotifySecrets {
    /// Try to pull secrets from KV-storage and fail otherwise
    pub fn from_secret_store(env: &Env) -> std::result::Result<SpotifySecrets, worker::Error> {
        console_log!("Accessing spotify api secrets");
        let spotify_client_id = env.secret("SPOTIFY_CLIENT_ID")?.to_string();
        let spotify_client_secret = env.secret("SPOTIFY_CLIENT_SECRET")?.to_string();
        let spotify_redirect_uri = env.secret("SPOTIFY_REDIRECT_URI")?.to_string();
        let spotify_refresh_token = env.secret("SPOTIFY_REFRESH_TOKEN")?.to_string();
        console_log!("Found all spotify api secrets");

        Ok(SpotifySecrets {
            spotify_client_id,
            spotify_client_secret,
            spotify_refresh_token,
            spotify_redirect_uri,
        })
    }
}

/// Performs all necessary token refreshing and creates an instance of AuthCodeSpotify for use in
/// API endpoints
async fn get_spotify_api_handle(
    env: &worker::Env,
) -> std::result::Result<AuthCodeSpotify, worker::Error> {
    // obtain spotify secrets and propogate error if failed
    let spotify_secrets = match SpotifySecrets::from_secret_store(env) {
        Ok(secrets) => secrets,
        Err(worker_error) => {
            console_error!("Failed to obtain spotify secrets. Panic");
            return Err(worker_error);
        }
    };

    let spotify_credentials = Credentials::new(
        &spotify_secrets.spotify_client_id,
        &spotify_secrets.spotify_client_secret,
    );

    console_log!("Successfully generated spotify credentials");

    let spotify_oauth = OAuth {
        redirect_uri: spotify_secrets.spotify_redirect_uri,
        ..Default::default()
    };

    console_log!("Created spotify oauth configuration");

    // create bogus token that will need to be refreshed, but contains refresh token
    let token = Token {
        access_token: "placeholder".to_string(),
        refresh_token: Some(spotify_secrets.spotify_refresh_token.clone()),
        expires_in: TimeDelta::new(3600, 0).expect("Failed to create trivial time delta"),
        expires_at: Some(DateTime::UNIX_EPOCH), // mark as expired
        scopes: Default::default(),
    };

    console_log!("Generated intentionally invalid spotify token");

    // initialize api handle from token
    let spotify = AuthCodeSpotify::from_token_with_config(
        token,
        spotify_credentials,
        spotify_oauth,
        rspotify::Config::default(),
    );

    // refresh the access token
    if let Err(e) = spotify.refresh_token().await {
        console_error!("Failed to refresh spotify token {}", e.to_string());
        panic!("Could not refresh token.");
    }

    console_log!("Successfully refreshed spotify token");

    Ok(spotify)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayTopTrack {
    pub track_name: String,
    pub track_name_link: String,
    pub track_artist: String,
    pub track_artist_link: String,
    pub track_album_art_url: String,
}

/// Takes in a stringified version of the top tracks because most of the time (when cached),
/// it will be returning that, and can generate the htmx by deserializing
pub fn generate_htmx(stringified_top_tracks: String) -> String {
    let Ok(top_tracks) = serde_json::from_str::<Vec<DisplayTopTrack>>(&stringified_top_tracks)
    else {
        console_error!(
            "Failed to deserialize top tracks.\nTOP_TRACKS_STRING: {}",
            stringified_top_tracks
        );
        return "[{}]".to_string();
    };

    // NOTE: As of now, this only returns the first 3 tracks & their information. This will throw
    // an error if it is less than 3, and won't utilize the tracks after 3. Using this simple
    // approach temporarily (famous last words)
    top_songs_to_templated_html(top_tracks)
}

/// Performs the templating and can adjust to how many songs are given as input
fn top_songs_to_templated_html(top_tracks: Vec<DisplayTopTrack>) -> String {
    // use only the number of songs to display
    let top_tracks_to_display = top_tracks
        .clone()
        .into_iter()
        .take(NUM_TOP_TRACKS_TO_DISPLAY)
        .collect::<Vec<DisplayTopTrack>>();

    let mut generated_html = r#"
    <div class="top-songs">
        <h1>Top Songs of the Month</h1>
        <div class="top-songs-item-list">
    "#
    .to_string();

    for track in top_tracks_to_display {
        let generated_top_songs_item = format!(
            r#"
            <div class="top-songs-item">
                <img src="{0}"/>
                <a href="{1}">{2}</a>
                <a href="{3}">{4}</a>
            </div>"#,
            track.track_album_art_url,
            track.track_name_link,
            track.track_name,
            track.track_artist_link,
            track.track_artist,
        );

        generated_html.push_str(&generated_top_songs_item);
    }

    generated_html.push_str(
        r#"
            </div>
        </div>
        "#,
    );

    generated_html
}
