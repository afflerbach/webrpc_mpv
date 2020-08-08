pub mod episodes;
pub mod series;
use crate::tmdb;
use rocket::response::content;
#[get("/scan")]
pub fn request_scan() -> content::Json<String> {
    scan_dir();
    let path_entries = get_first_level();
    let settings = settings::init();
    let mut test = Vec::new();
    let mut tmdb_response;
    for entry in &path_entries {
        tmdb_response = tmdb::tmdb::search(entry.to_string());
        for mut result in tmdb_response.results {
            if !check_tmdb_id(result.id) {
                let file_path: Option<String> = Some(format!("{}{}", settings.scan_dir, entry));
                result.file_path = file_path;
                test.push(result);
            }
        }
    }

    let me = serde_json::to_string(&test).unwrap();

    content::Json(me.to_string())
}

use crate::settings;
use std::{fs, io};
fn get_first_level() -> Vec<String> {
    let settings = settings::init();
    println!("{}", settings.scan_dir);
    let base_path = settings.scan_dir.clone();
    let mut entries = fs::read_dir(base_path)
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, io::Error>>()
        .unwrap();

    entries.sort();
    let path_part = settings.scan_dir.clone();
    let mut stack = Vec::new();
    for entry in &entries {
        let foo = entry.display().to_string().replace(&path_part, "");
        stack.push(foo)
    }

    return stack;
}
#[derive(Serialize, Deserialize)]
pub struct LibraryRequest {
    pub tmdb_id: i32,
    pub path: String,
}

use rocket_contrib::json::Json;
#[post("/add", data = "<request_content>")]
pub fn request_add_serie(request_content: Json<LibraryRequest>) -> content::Json<String> {
    fn first<T>(v: &Vec<T>) -> Option<&T> {
        v.first()
    }
    let external_id = tmdb::tmdb::get_external_id(request_content.tmdb_id);
    println!("External id: {}", external_id.tvdb_id);

    let serie_information = tmdb::tmdb::find_by_external_id(external_id.tvdb_id);

    let info_vec = &first(&serie_information.tv_results).unwrap();
    let serie_info = NewSerie {
        imagepath: &info_vec.poster_path.as_ref().unwrap(),
        tmdb_id: &request_content.tmdb_id,
        title: &info_vec.name,
        description: &info_vec.overview.as_ref().unwrap(),
    };

    if !serie_info.check_serie() {
        let connection = mpv_webrpc::establish_connection();
        use mpv_webrpc::schema::serie;
        let _insert_result = diesel::insert_into(serie::table)
            .values(&serie_info)
            .execute(&connection);

        sync_episodes(request_content.path.clone(), request_content.tmdb_id);
    }

    let test = json!({
        "data": "ok",
        "message": "",
        "request_id": 0
    });
    content::Json(test.to_string())
}
fn sync_season(tmdb_id_to_insert: i32, season_id_to_insert: i32) {
    use mpv_webrpc::schema::season;
    let season_in = tmdb::tmdb::tv_season_get_details(tmdb_id_to_insert, season_id_to_insert);
    let season_info = NewSeason {
        season_id: &season_id_to_insert,
        tmdb_id: &tmdb_id_to_insert,
        title: &season_in.name,
        imagepath: &season_in.poster_path,
        description: &season_in.overview,
    };

    let connection = mpv_webrpc::establish_connection();
    let _insert_result = diesel::insert_into(season::table)
        .values(&season_info)
        .execute(&connection);
}

fn sync_episodes(path: String, tmdb_id: i32) {
    let mkv_pattern = format!("{}/**/*.mkv", path);
    let mp4_pattern = format!("{}/**/*.mp4", path);

    use mpv_webrpc::schema::episode;
    for entry in glob(&mkv_pattern)
        .unwrap()
        .chain(glob(&mp4_pattern).unwrap())
    {
        match entry {
            Ok(path) => {
                println!("fetch for episodes and season in path: {:?}", path);
                let file_name = &path.clone().into_os_string().into_string().unwrap();
                let captures = parsing_season_and_episode(file_name);
                //
                if captures.is_none() {
                    println!("Pattern not match");
                    continue;
                }

                let unwrap_cap = captures.unwrap();
                let s = unwrap_cap
                    .get(1)
                    .map_or("", |m| m.as_str())
                    .replace("S", "");
                let season: i32 = s.replace("s", "").parse::<i32>().unwrap();

                let e = unwrap_cap
                    .get(2)
                    .map_or("", |m| m.as_str())
                    .replace("E", "");
                let episode: i32 = e.replace("e", "").parse::<i32>().unwrap();

                let season_info = NewSeason {
                    season_id: &season,
                    tmdb_id: &tmdb_id,
                    title: &"".to_string(),
                    imagepath: &"".to_string(),
                    description: &"".to_string(),
                };

                if !season_info.check_season() {
                    println!("insert season information for {} {}", tmdb_id, season);
                    sync_season(tmdb_id, season);
                }

                let mut epi_info = NewEpisode {
                    path: file_name,
                    serie_id: &tmdb_id,
                    season_id: &season,
                    episode_id: &episode,
                    tmdb_id: &tmdb_id,
                    title: &"".to_string(),
                    description: &"".to_string(),
                };
                if !epi_info.check_episode() {
                    let episode_info =
                        tmdb::tmdb::tv_episodes_get_details(tmdb_id, season, episode);

                    epi_info.title = &episode_info.name;
                    epi_info.description = &episode_info.overview;

                    let connection = mpv_webrpc::establish_connection();
                    let _insert_result = diesel::insert_into(episode::table)
                        .values(&epi_info)
                        .execute(&connection);
                    println!("insert episode info {:?}", episode_info);
                }
            }
            Err(e) => println!("{:?}", e),
        }
    }
}

use diesel::prelude::*;
use mpv_webrpc::models::*;
fn check_tmdb_id(id_to_check: i32) -> bool {
    let ignored = NewIgnored {
        tmdb_id: &id_to_check.clone(),
    };

    let serie_info = NewSerie {
        tmdb_id: &id_to_check.clone(),
        description: &"".to_string(),
        title: &"".to_string(),
        imagepath: &"".to_string(),
    };

    let is_ignored = ignored.is_ignored();
    let serie_exists = serie_info.check_serie();

    if serie_exists || is_ignored {
        return true;
    }

    return false;
}
#[derive(Serialize, Deserialize)]
pub struct LibraryIgnoreRequest {
    pub tmdb_id: i32,
}

#[post("/ignore", data = "<request_content>")]
pub fn request_ignore_serie(request_content: Json<LibraryRequest>) -> content::Json<String> {
    use mpv_webrpc::schema::ignored;
    let connection = mpv_webrpc::establish_connection();

    let ignore_serie = NewIgnored {
        tmdb_id: &request_content.tmdb_id,
    };

    let insert_result = diesel::insert_into(ignored::table)
        .values(&ignore_serie)
        .execute(&connection);
    let message;

    match insert_result {
        Ok(v) => message = format!("{:?}", v),
        Err(e) => message = format!("{:?}", e),
    }

    let test = json!({
        "data": "ok",
        "message": message,
        "request_id": 0
    });
    content::Json(test.to_string())
}

use glob::glob;
fn scan_dir() {
    let settings = settings::init();
    let pattern = format!("{}/**/*.mkv", settings.scan_dir);

    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                parsing_season_and_episode(&path.clone().into_os_string().into_string().unwrap());
            }
            Err(e) => println!("{:?}", e),
        }
    }
}

extern crate lazy_static;
use lazy_static::lazy_static;
use regex::Regex;
fn parsing_season_and_episode(text: &str) -> Option<regex::Captures> {
    println!("{}", text);
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(S\d{1,2}|s\d{1,2})(E\d{1,2}|e\d{1,2})").unwrap();
    }

    RE.captures(text)
}
