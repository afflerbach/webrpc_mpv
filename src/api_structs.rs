
use std::collections::HashMap;

#[derive(Serialize)]
pub struct TemplateContext {
    pub items: HashMap<String, String>
}


#[derive(Serialize, Deserialize)]
pub struct PlaylistControl {
pub    value : String
}

#[derive(Serialize, Deserialize)]
pub struct VolumeControl {
pub    value : String
}

#[derive(Serialize, Deserialize)]
struct Response {
    data: String,
    error: String,
    request_id: i32,
}


#[derive(Serialize, Deserialize)]
struct VolumResponse {
    data: String,
    error: String,
    request_id: i32,
}

#[derive(FromForm)]
pub struct UrlForm {
   pub  target: String,
   pub  action: String,
}

