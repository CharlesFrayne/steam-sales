use serde::{Deserialize, Serialize};
use std::env;

const BASE_URL: &str = "https://api.steampowered.com/";
const OWNED: &str = "IPlayerService/GetOwnedGames/v1";
const INFO: &str = "https://store.steampowered.com/api/appdetails";

#[derive(Deserialize, Serialize)]
struct LibraryResponse {
    response: Library,
}

#[derive(Deserialize, Serialize)]
struct Library {
    games: Vec<Game>,
}

#[derive(Deserialize, Serialize)]
struct Game {
    appid: u64,
    name: String,
    playtime_forever: u64,
}

#[derive(Deserialize, Serialize)]
struct PriceOverview {
    initial: u64,
    r#final: u64,
    discount_percent: u64,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let api_key = &args[1];
    let steam_id = &args[2];
    let library_body = request_library(api_key, steam_id).await.unwrap();
    let library: LibraryResponse = serde_json::from_str(library_body.as_str()).unwrap();

    let mut played_games = library.response.games;
    played_games.retain(|g| g.playtime_forever > 0);

    let prices_body = request_prices(api_key, &played_games)
        .await
        .expect("couldn't even get a price response");
    let mut prices_json: serde_json::Value =
        serde_json::from_str(prices_body.as_str()).expect("really couldn't get the price overview");

    println!(
        "{}, {}, {}, {}, {}",
        "id", "name", "minutes played", "full price", "discounted price"
    );
    for g in played_games.iter() {
        let appid = g.appid;
        let mut game_data = prices_json[appid.to_string().as_str()].take();
        let po: PriceOverview = match game_data["data"] {
            serde_json::Value::Object(_) => {
                serde_json::from_value(game_data["data"]["price_overview"].take()).unwrap()
            }
            _ => PriceOverview {
                initial: 0,
                r#final: 0,
                discount_percent: 0,
            },
        };

        if po.discount_percent > 0 {
            println!(
                "{}, {}, {}, {}, {}",
                g.appid, g.name, g.playtime_forever, po.initial, po.r#final
            );
        }
    }

    Ok(())
}

async fn request_library(api_key: &str, steam_id: &str) -> Result<String, reqwest::Error> {
    let key_param = ("key", api_key);
    let id_param = ("steamid", steam_id);
    let info_param = ("include_appinfo", "true");
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/{}", BASE_URL, OWNED))
        .query(&[key_param, id_param, info_param])
        .send()
        .await?;
    let body: String = resp.text().await?;
    Ok(body)
}

async fn request_prices(api_key: &str, games: &Vec<Game>) -> Result<String, reqwest::Error> {
    let key_param = ("key", api_key);
    let appids_csv = games
        .iter()
        .map(|g| g.appid.to_string())
        .collect::<Vec<String>>()
        .join(",");
    let ids_param = ("appids", appids_csv.as_str());
    let filter_param = ("filters", "price_overview");
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}", INFO))
        .query(&[key_param, ids_param, filter_param])
        .send()
        .await?;
    let body: String = resp.text().await?;
    Ok(body)
}
