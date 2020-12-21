pub mod mpv {

    extern crate execute;
    use crate::api_structs::VolumeControl;
    use crate::settings;
    use std::io::prelude::*;
    use std::os::unix::net::UnixStream;
    use std::process::Command;
    use serde_json::json;
    use serde::{Serialize, Deserialize};

    pub fn event_play_from_list(target: String) -> std::io::Result<String> {
        let tjson = json!({ "command": ["loadlist", format!("{}",target)] });
        write_to_socket(tjson.to_string() + "\n")



    }

    pub fn event_play(target: String) -> &'static str {
        let mut mpv = Command::new("mpv");
        let settings = settings::init();
        let ipc_param = format!("--input-ipc-server={}", settings.socket);
        mpv.arg(target).arg(ipc_param).spawn().expect("OK");
        "Hello, world!"
    }

    pub fn event_resume() -> Property {
        let command = json!({ "command": ["set_property", String::from("pause"), false] });
        let result :Value= serde_json::from_str(write_to_socket(command.to_string() + "\n").unwrap().as_str()).unwrap();

        let me = Property {
            error : result["error"].to_string().replace("\"", ""),
            data : result["data"].to_string()
        };

        return me;
    }

    pub fn event_load(target: String) -> Property {
        let command = json!({ "command": ["loadfile", format!("{}",target)] });
        let result :Value= serde_json::from_str(write_to_socket(command.to_string() + "\n").unwrap().as_str()).unwrap();
        let me = Property {
            error : String::from(""),
            data : result["event"].to_string()
        };
        return me;
    }
    pub fn event_pause() -> Property {
        let command = json!({ "command": ["set_property", String::from("pause"), true] });
        let result :Value= serde_json::from_str(write_to_socket(command.to_string() + "\n").unwrap().as_str()).unwrap();

        let me = Property {
            error : result["error"].to_string().replace("\"", ""),
            data : result["data"].to_string()
        };

        return me;
    }
    pub fn event_quit() -> std::io::Result<String> {
        let tjson = json!({ "command": ["quit"] });
        write_to_socket(tjson.to_string() + "\n")
    }

    fn update_video_status() {
        use mpv_webrpc::models::NewVideoStatus;
        let path = event_property("path".to_string(), None);

        if path.error == String::from("success") {

            let time_json:String = event_property("time-pos".to_string(), None).data;
            let path_json:String = event_property("path".to_string(), None).data;

            // serde json supports only f64 - diesel supports only f32 for fields - *sigh*
            let time : f64= time_json.parse().unwrap();
            let convert = time as f32;
            let video_status = NewVideoStatus {
                path: &path_json.replace("\"", ""),
                time: &convert,
            };
            video_status.upsert();
        }
    }

    use serde_json::Value;
    pub fn event_stop() -> Property {
        update_video_status();
        let command = json!({ "command": ["stop"] });
        let result :Value= serde_json::from_str(write_to_socket(command.to_string() + "\n").unwrap().as_str()).unwrap();
        let me = Property {
            error : String::from("success"),
            data : result["event"].to_string()
        };
        return me;
    }
    pub fn event_volume() -> Property {
        event_property(String::from("volume"), None)
    }

    pub fn event_volume_change(volume_control: VolumeControl) -> Property {
        event_property(String::from("volume"), Some(volume_control.value))



    }

    pub fn event_property(propery: String, value:Option<String>) -> Property {
        let command: Value = match value {
            None => {
                json!({ "command": ["get_property", propery] })
            }, 
            Some(value) => {
                json!({ "command": ["set_property", propery, value] })
            },
        };
        let result :Value= serde_json::from_str(write_to_socket(command.to_string() + "\n").unwrap().as_str()).unwrap();

        let me = Property {
            error : result["error"].to_string().replace("\"", ""),
            data : result["data"].to_string()
        };

        return me;
    }

    #[derive( Debug, Serialize, Deserialize)]
    pub struct Property {
        pub error : String,
        pub data : String
    }

    pub fn event_get_property(propery: String) -> Property {
        let tjson = json!({ "command": ["get_property", propery] });
        let result :Value= serde_json::from_str(write_to_socket(tjson.to_string() + "\n").unwrap().as_str()).unwrap();

        let me = Property {
            error : result["error"].to_string().replace("\"", ""),
            data : result["data"].to_string()
        };

        return me;
    }

    fn print_type_of<T>(_: &T) {
        println!("{}", std::any::type_name::<T>())
    }
    pub fn init() {
        let settings = settings::init();

        let mut mpv = Command::new("mpv");
        let ipc_param = format!("--input-ipc-server={}", settings.socket);
        println!("Starting parameter for mpv: {}", ipc_param);
        mpv.arg("--idle=yes")
            .arg(ipc_param)
            .arg("--hwdec=mmal-copy")
            .arg("--fs=yes")
            .arg("--vo=gpu")
            .spawn()
            .expect("OK");
    }

    pub fn write_to_socket(content: String) -> std::io::Result<String> {
        let settings = settings::init();
        let socket = settings.socket;
        let mut stream = match UnixStream::connect(&socket) {
            Err(e) => panic!("could not connect to socket {} - {}", e, &socket),
            Ok(stream) => stream,
        };

        stream.write_all(&content.as_bytes())?;
        let mut buf = [0; 1024];
        let count = stream.read(&mut buf).unwrap();
        let response = String::from_utf8(buf[..count].to_vec()).unwrap();

        Ok(response)
    }
}
