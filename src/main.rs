use clap::Parser;
use futures::FutureExt;
use serde_json::Value;
use std::error::Error;
use std::{collections::HashMap, fs::read_to_string, fs::write};
use tokio::io::{AsyncBufReadExt, BufReader, Stdin};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
mod types;
use futures::future::join_all;
use std::result::Result as StdResult;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use types::{Meta, MouseHandler};

fn get_handler(my_type: &str) -> Box<dyn types::Handler> {
    match my_type {
        "date" => Box::new(types::Date),
        "battery" => Box::new(types::Battery),
        "wifi" => Box::new(types::Wifi),
        "volume" => Box::new(types::Volume),
        "quote" => Box::new(types::Quote),
        "current" => Box::new(types::CurrentProgram),
        "bgchange" => Box::new(types::BgChanger),
        _ => Box::new(types::Noop),
    }
}

async fn render(mut chan: Receiver<Vec<types::Out>>) {
    println!("{}", "{\"version\":1, \"click_events\":true}");
    println!("[");
    println!("[],");
    tokio::task::spawn(async move {
        while let Some(msg) = chan.recv().await {
            let out_json = serde_json::to_string(&msg);
            match out_json {
                Ok(out) => println!("{},", out),
                Err(_) => (),
            }
        }
    });
}

fn get_mouse_handler(x: &str) -> Box<dyn MouseHandler> {
    match x {
        "wifi" => Box::new(types::WifiClick),
        "volume" => Box::new(types::VolumeClick),
        _ => Box::new(types::MouseNoop),
    }
}

fn mouse_listener(chan: Sender<Box<dyn types::MouseHandler>>, reader: BufReader<Stdin>) {
    let mut lines = reader.lines();

    tokio::task::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            let line = if line.chars().nth(0).unwrap_or(' ') == ',' {
                line.trim_start_matches(',')
            } else {
                line.as_str()
            };
            //let line = line.trim_start_matches(',');
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                let instance = &value["instance"];
                let inst = instance.as_str().unwrap_or("");
                let h = get_mouse_handler(inst);
                let _ = chan.send(h).await;
            }
        }
    });
}

fn write_state(mut chan: Receiver<HashMap<String, Meta>>, out_path: String, buffer_size: i32) {
    tokio::task::spawn(async move {
        let mut counter = 0;
        while let Some(msg) = chan.recv().await {
            if counter % buffer_size == 0 {
                let out_json = serde_json::to_string(&msg);
                match out_json {
                    Ok(x) => {
                        let _ = write(out_path.as_str(), x);
                    }
                    Err(_) => (),
                };
            }
            counter = (counter + 1) % buffer_size;
        }
    });
}

#[derive(Parser)]
#[command(name = "swaybar")]
#[command(author = "thomas@gebert.app")]
#[command(version = "1.0")]
#[command(about = "nada")]
pub struct Args {
    #[arg(short, long)]
    pub config: String,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() -> StdResult<(), Box<dyn Error>> {
    let stdin = tokio::io::stdin(); // 
    let reader = BufReader::new(stdin);
    let args = Args::parse();
    let path = args.config;
    let config_str = read_to_string(path)?;
    let config: types::Config = serde_json::from_str(config_str.as_str())?;
    let init_state_str = match read_to_string(config.persist.path.to_string()) {
        Ok(my_str) => my_str,
        Err(_) => "{}".to_string(),
    };
    let mut state: HashMap<String, Meta> = serde_json::from_str(init_state_str.as_str())?;

    let poll_time = Duration::from_millis(config.poll_time);
    let (render_sender, render_receiver) = tokio::sync::mpsc::channel::<Vec<types::Out>>(5);
    render(render_receiver).await;

    let (state_sender, state_receiver) = tokio::sync::mpsc::channel::<HashMap<String, Meta>>(5);

    let (mouse_sender, mut mouse_receiver) =
        tokio::sync::mpsc::channel::<Box<dyn types::MouseHandler>>(10);
    write_state(
        state_receiver,
        config.persist.path,
        config.persist.buffer_size,
    );
    let mut futures = HashMap::<String, JoinHandle<Option<HashMap<String, String>>>>::new();

    mouse_listener(mouse_sender, reader);
    loop {
        if let Some(Some(msg)) = mouse_receiver.recv().now_or_never() {
            tokio::spawn(async move {
                let _ = msg.click_handle().await;
            });
        }
        let loop_begin = std::time::Instant::now();
        let mut futs = Vec::new();

        for module_config in &config.modules {
            let timeout_ms = module_config.timeout.unwrap_or(config.default_timeout);
            let timeout = Duration::from_millis(timeout_ms);
            let default = Meta {
                is_processing: false,
                start_time: Duration::ZERO,
                data: HashMap::new(),
            };
            let mut state = state.get(&module_config.name).unwrap_or(&default).clone();
            let ttl = Duration::from_millis(module_config.ttl);
            let name = module_config.name.clone();
            let display = module_config.display.unwrap_or(true);

            let old_fut = futures.remove(&name);

            let fin = async move {
                let name = name.as_str();
                let handler1 = get_handler(name);
                let handler2 = get_handler(name);
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");
                let expire_time = state.start_time + ttl;
                let mut my_f = if !state.is_processing && now > expire_time {
                    if let Some(f) = old_fut {
                        f.abort();
                    }

                    let fut = tokio::spawn(async move {
                        let f = handler1.handle().await;
                        match f {
                            Ok(ff) => Some(ff),
                            Err(_) => None,
                        }
                    });
                    state.is_processing = true;
                    state.start_time = now;
                    Some(fut)
                } else {
                    old_fut
                };

                let new_fut = match my_f.as_mut().and_then(|f| f.now_or_never()) {
                    Some(Ok(res)) => {
                        if let Some(res) = res {
                            state.data.extend(res);
                            state.is_processing = false;
                        }
                        None
                    }
                    _ => {
                        let elapsed = now.checked_sub(state.start_time).unwrap_or(Duration::ZERO);
                        if state.is_processing {
                            if elapsed < timeout {
                                my_f
                            } else {
                                state.is_processing = false;
                                state.start_time = Duration::ZERO;

                                match my_f {
                                    Some(x) => x.abort(),
                                    None => (),
                                }
                                None
                            }
                        } else {
                            my_f
                        }
                    }
                };

                let out = if display {
                    Some(handler2.render(&state.data))
                } else {
                    None
                };

                (name.to_string(), state, out, new_fut)
            };
            futs.push(fin);
        }

        let values = join_all(futs).await;

        let out_objs: Vec<types::Out> = values
            .into_iter()
            .filter_map(|(name, meta, out_str, new_fut)| {
                state.insert(name.clone(), meta);
                if let Some(f) = new_fut {
                    futures.insert(name.clone(), f);
                }

                out_str.map(|f| types::Out {
                    name: name.clone(),
                    instance: name,
                    full_text: f,
                })
            })
            .collect();

        render_sender.send(out_objs).await?;
        state_sender.send(state.clone()).await?;

        let elapsed = loop_begin.elapsed();
        let wait_time = poll_time.checked_sub(elapsed).unwrap_or(Duration::ZERO);
        tokio::time::sleep(wait_time).await;
    }
}
