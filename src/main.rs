use std::{collections::HashMap, fs::read_to_string};
use std::error::Error;
use std::result::Result as StdResult;
use std::time::Duration;
use tokio::process::Command;
use async_trait::async_trait;
use futures::future::join_all;
use chrono::{Datelike, Local, Timelike};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};


fn bat_status_icons(n: &str) -> &'static str {
   match n {
       "full" => "🟢",
       "charging" => "⚡",
       "notcharging" => "🔌",
       "discharging" => "🔋",
       _ => ""
   }
}

fn wifi_status_icons(n : &str) -> &'static str {

    match n {
        "connected" => "📶",
        "disconnected" => "❌",
        _ => ""
    }
}

fn month_abbr(n: u32) -> &'static str {
    match n {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???", // or "" or panic!("bad month")
    }
}

struct Meta {
   is_processing: bool,
   start_time: Duration,
   data: HashMap<String,String>
}

#[async_trait]
trait Handler {
    async fn handle(&self) -> StdResult<HashMap<String,String>, Box<dyn Error>>;
    fn render(&self, i: HashMap<String,String>)-> String; 
}

struct Noop;

#[async_trait]
impl Handler for Noop {
    async fn handle(&self) -> StdResult<HashMap<String,String>, Box<dyn Error>> {
        let mut out_hash = HashMap::new();
        out_hash.insert("".to_string(),"".to_string());
        Ok(out_hash)
    }
    fn render(&self, i : HashMap<String, String>) -> String {
        "".to_string()
    }
}

struct Battery;

#[async_trait]
impl Handler for Battery {
    async fn handle(&self) -> StdResult<HashMap<String,String>, Box<dyn Error>> {
        let bat_path = "/sys/class/power_supply/BAT0";
        let cap_path = format!("{}/capacity", bat_path);
        let stat_path = format!("{}/status", bat_path);
        let cap_string = std::fs::read_to_string(cap_path)?.trim().replace("\"", "");
        let stat_string = std::fs::read_to_string(stat_path)?.trim().to_lowercase().replace(" ", "");

        let mut out_map = HashMap::new(); 
        out_map.insert("capacity".to_string(), cap_string.to_string());
        out_map.insert("status".to_string(), stat_string.to_string());
        Ok(out_map) 
    }

    fn render(&self, i : HashMap<String, String>) -> String {
        let empty = "".to_string();
        let cap = i.get("capacity").unwrap_or(&empty);
        let stat = i.get("status").unwrap_or(&empty).as_str();
        format!("{} {}%", bat_status_icons(stat) , cap)
    }
}

struct Wifi;

#[async_trait]
impl Handler for Wifi {
    async fn handle(&self) -> StdResult<HashMap<String,String>, Box<dyn Error>> {

        let wifi_cmd = Command::new("iw").arg("dev").output().await?;
        let s : Vec<String> = String::from_utf8_lossy(&wifi_cmd.stdout).lines().map(|s| s.trim().to_string()).collect();
        let interface : &str = s[5].split(" ").last().unwrap_or("");

        let connected_cmd = Command::new("iw").arg(interface).arg("link").output().await?;
        let s2 = String::from_utf8_lossy(&connected_cmd.stdout).find("Connected");

        let is_connected = match s2 {
            Some(_) => true,
            None => false 
        };

        let connect_status = if is_connected {"connected"} else {"disconnected"};

        let out_map : HashMap<String, String>  = 
            [("connect_status", connect_status)]
            .iter()
                .map(|(k,v)| (k.to_string(), v.to_string()))
                .collect();
        //out_map.insert("connect_status".to_string(), connect_status.to_string());

        Ok(out_map)
    }

    fn render(&self, i : HashMap<String, String>) -> String {

        let EMPTY = "".to_string();
        let connected = i.get("connect_status").unwrap_or(&EMPTY);
        format!("{}", wifi_status_icons(&connected))
    }
}

struct Volume;
#[async_trait]
impl Handler for Volume {
    async fn handle(&self) -> StdResult<HashMap<String,String>, Box<dyn Error>> {
        let SPACE = " ".to_string();
        let is_muted_cmd = Command::new("pactl").arg("get-sink-mute").arg("@DEFAULT_SINK@").output().await?;
        let vol_info_cmd = Command::new("pactl").arg("get-sink-volume").arg("@DEFAULT_SINK@").output().await?; 
        let vol_info_str = String::from_utf8_lossy(&vol_info_cmd.stdout);
        let vol_info : Vec<String> = vol_info_str.split(&SPACE).map(|i| i.to_string()).collect();
        let is_muted_raw_str = String::from_utf8_lossy(&is_muted_cmd.stdout);
        let is_muted_lower = is_muted_raw_str.to_lowercase();
        let is_muted_str = is_muted_lower.split(&SPACE).map(|x| x.trim()).last(); 

        let is_muted = match is_muted_str {
            Some("yes") => "muted",
            _ => "not muted" 
        }.to_string();

        let vol_level = if vol_info[5] == "/" {vol_info[4].to_string()} else {vol_info[5].to_string()}.replace("%","");
        let out_map : HashMap<String, String> = [("volume_level", vol_level), ("is_muted", is_muted)].iter().map(|(k,v)| (k.to_string(), v.to_string())).collect(); 

        Ok(out_map)
    }

    fn render(&self, i : HashMap<String, String>) -> String {
        let default_muted = "default_muted".to_string();
        let is_muted = i.get("is_muted").unwrap_or(&default_muted);
        let small_speaker_cutoff = 40;
        let mid_speaker_cutoff = 80; 
        let default_vol = "50".to_string();
        let vol_level_str = i.get("volume_level").unwrap_or(&default_vol); 

        let vol_level : i32 = vol_level_str.parse().unwrap_or(50); 

        let icon = if is_muted == "muted" {
            "🔇"
        } else if vol_level < small_speaker_cutoff {
            "🔈"
        } else if vol_level < mid_speaker_cutoff {
            "🔉"
        } else {
            "🔊"
        };

        format!("{}{}%", icon, vol_level_str)

    }
}


struct Date;

#[async_trait]
impl Handler for Date{
    async fn handle(&self) -> StdResult<HashMap<String,String>, Box<dyn Error>> {
        let now = Local::now();
        let weekday = now.weekday();
        let day = now.day();
        let month = month_abbr(now.month());
        let seconds = format!("{:02}", now.second());
        let mod_hour = if now.hour() % 12 == 0 {12} else {now.hour() % 12};
 
        let hour = format!("{:02}", mod_hour);  
        let minutes = format!("{:02}", now.minute());
        let mut out_hash = HashMap::new();
        out_hash.insert("weekday".to_string(), weekday.to_string());
        out_hash.insert("day".to_string(), day.to_string());
        out_hash.insert("month".to_string(), month.to_string());
        out_hash.insert("seconds".to_string(), seconds.to_string());
        out_hash.insert("minutes".to_string(), minutes.to_string());
        out_hash.insert("hour".to_string(), hour.to_string());
        Ok(out_hash)
    }

    fn render(&self, i : HashMap<String, String>) -> String {
        static EMPTY: String = String::new();

        let hour = i.get("hour").unwrap_or(&EMPTY);
        let minute = i.get("minutes").unwrap_or(&EMPTY);
        let day = i.get("day").unwrap_or(&EMPTY);
        let month = i.get("month").unwrap_or(&EMPTY);
        let weekday = i.get("weekday").unwrap_or(&EMPTY);
        let seconds = i.get("seconds").unwrap_or(&EMPTY);
        format!("{} {} {} {}:{} {}", weekday, month, day, hour, minute, seconds )
    }
}

fn get_handler(my_type: &str) -> Box<dyn Handler> {
    match my_type {
        "date" => Box::new(Date),
        "battery" => Box::new(Battery),
        "wifi" => Box::new(Wifi),
        "volume" => Box::new(Volume),
        _ => Box::new(Noop)
    }
}


#[derive(Serialize, Deserialize)]
struct Out {
   name : String, 
   instance: String,
   full_text: String
}

#[derive(Serialize, Deserialize)]
struct PersistConfig  {
    path: String,
    buffer_size : i32
}

#[derive(Serialize, Deserialize)]
struct ModuleConfig {
   name: String,
   ttl: u64
}

#[derive(Serialize, Deserialize)]
struct Config {
   poll_time : i32,
   default_timeout: i32, 
   persist : PersistConfig, 
   modules : Vec<ModuleConfig>
}

#[tokio::main]
async fn main() -> StdResult<(), Box<dyn Error>> {
    let poll_time_ms = Duration::from_millis(50);
    //let handlers = [ "wifi", "battery", "volume", "date"].map(|i| (i, get_handler(i)));
    let mut state : HashMap<String, Meta>= HashMap::new(); 
    let config_str = read_to_string("swaybar-config.json")?;
    let config : Config =  serde_json::from_str(config_str.as_str())?;

    println!("{}","{\"version\":1, \"click_events\":true}");
    println!("[");
    println!("[],");
    loop {
        let loop_begin = std::time::Instant::now();
        let loop_now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO); 
        
        let futs = config.modules.iter().map(|i| async {
            let now = SystemTime::now(); 
            let epoch = now.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
            let ttl = Duration::from_millis(i.ttl); 
            let ttl_expire = now + ttl; 
            let name = &i.name;
            let handler = get_handler(i.name.as_str());
            let fut = handler.handle();
            let res = tokio::select! {
                res = fut => res,
                else => {
                    let f = 
                        Meta {is_processing: true, 
                            start_time: epoch,
                            data : HashMap::new()
                        }; 
                    Ok(state.get(&name.to_string()).unwrap_or(&f).data)

                }
            };
            match res {
                Ok(hm) => (name.to_string(), hm.clone(), handler.render(hm)),
                Err(_e) => (name.to_string(), HashMap::new(), "".to_string())
            }
        });

        let values = join_all(futs).await; 

        let out_objs: Vec<Out> = values.iter().map(|(name, data, out_str)|{
            let f = Meta {
                start_time: loop_now,
                data : HashMap::new(),
                is_processing: false
            };
            let curr_state = state.entry(name.to_string()).or_insert_with(|| f);
            data.iter().for_each(|(k, v)| {
                curr_state.data.insert(k.to_string(), v.to_string());
            });
            Out {
                name: name.clone(),
                instance: name.clone(),
                full_text: out_str.to_string()
            }
        }).collect();

        let out_json = serde_json::to_string(&out_objs)?;
        println!("{},", out_json);
        let elapsed = loop_begin.elapsed();
        let wait_time = poll_time_ms.checked_sub(elapsed).unwrap_or(Duration::ZERO);
        tokio::time::sleep(wait_time).await;
    }
}




