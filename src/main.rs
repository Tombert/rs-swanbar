use tokio::io::{AsyncBufReadExt, BufReader, Stdin};
use tokio::sync::mpsc::{channel, Sender, Receiver};
use tokio::task::JoinHandle;
use std::sync::{Arc, Mutex};
use serde_json::Value;
use futures::FutureExt;
use clap::Parser;
use std::{collections::HashMap, fs::read_to_string, fs::write};
use std::error::Error;
mod types;
use std::result::Result as StdResult;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use futures::future::join_all;
use types::{Meta, MouseHandler};

fn get_handler(my_type: &str) -> Box<dyn types::Handler> {
    match my_type {
        "date" => Box::new(types::Date),
        "battery" => Box::new(types::Battery),
        "wifi" => Box::new(types::Wifi),
        "volume" => Box::new(types::Volume),
        "quote" => Box::new(types::Quote),
        "current" => Box::new(types::CurrentProgram),
        _ => Box::new(types::Noop)
    }
}

async fn render(mut chan : Receiver<Vec<types::Out>>) {
    println!("{}","{\"version\":1, \"click_events\":true}");
    println!("[");
    println!("[],");
    tokio::task::spawn(async move {
        while let Some(msg) = chan.recv().await {
            let out_json = serde_json::to_string(&msg);
            match out_json {
                Ok(out) => println!("{},", out),
                Err(_) => ()
            }
        }
    });
}

fn get_mouse_handler(x : &str) -> Box<dyn MouseHandler> {
    match x {
        "wifi" => Box::new(types::WifiClick),
        "volume" => Box::new(types::VolumeClick),
        _ => Box::new(types::MouseNoop)
    }

}

async fn mouse_listener(mut chan : Sender<Box<dyn types::MouseHandler>>, reader: BufReader<Stdin>) {
    let mut lines = reader.lines();

    tokio::task::spawn(async move {
         while let Ok(Some(line)) = lines.next_line().await {
             if let Ok(value) = serde_json::from_str::<Value>(&line){
                 let instance = &value["instance"];
                 let inst = instance.as_str().unwrap_or("");
                 let h = get_mouse_handler(inst);
                 let _ = chan.send(h).await;
             }
         }
    });

}

async fn write_state(mut chan: Receiver<HashMap<String,Meta>>, out_path : String, buffer_size: i32) {
    tokio::task::spawn(async move {
        let mut counter = 0; 
        while let Some(msg) = chan.recv().await {
            if counter % buffer_size == 0 {
                let out_json = serde_json::to_string(&msg);
                match out_json {
                    Ok(x) => {write(out_path.as_str(), x);},
                    Err(_) => ()
                };
            }
            counter= (counter + 1) % buffer_size; 
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
    pub config: Option<String>,
}


#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> StdResult<(), Box<dyn Error>> {
    let stdin = tokio::io::stdin(); // 
    let reader = BufReader::new(stdin);
    let args = Args::parse();
    let path = args.config.unwrap_or("swaybar-config-new.json".to_string());
    let config_str = read_to_string(path)?;
    let config : types::Config =  serde_json::from_str(config_str.as_str())?;
    let init_state_str = match read_to_string(config.persist.path.to_string()) {
        Ok(my_str) => my_str,
        Err(_) => "{}".to_string()
    };
    let mut state : HashMap<String, Meta> = serde_json::from_str(init_state_str.as_str())?;

    let poll_time_ms = Duration::from_millis(config.poll_time);
    //let mut state : HashMap<String, types::Meta>= HashMap::new();
    let (render_sender, render_receiver) = tokio::sync::mpsc::channel::<Vec<types::Out>>(20);
    render(render_receiver).await;

    let (state_sender, state_receiver) = tokio::sync::mpsc::channel::<HashMap<String,Meta>>(20);

    let (mouse_sender, mut mouse_receiver) = tokio::sync::mpsc::channel::<Box<dyn types::MouseHandler>>(10);
    write_state(state_receiver, config.persist.path, config.persist.buffer_size).await;
    let futures = Arc::new(Mutex::new(HashMap::<String, JoinHandle<HashMap<String, String>>>::new()));

    mouse_listener(mouse_sender, reader).await;
    loop {
        if let Some(Some(msg)) = mouse_receiver.recv().now_or_never() {
            tokio::spawn (async move {
                let _ = msg.click_handle().await;
            });
        }
        let loop_begin = std::time::Instant::now();

        
        let futs = config.modules.iter().map(|module_config| {
            let default = Meta {
                is_processing : false,
                start_time : Duration::ZERO,
                data: [("month", "BLAH")].iter().map(|(k,v)|(k.to_string(), v.to_string())).collect()

            };
            let begin_state = state.get(&module_config.name).unwrap_or(&default).clone();
            let handler1 = get_handler(module_config.name.as_str());
            let handler2 = get_handler(module_config.name.as_str());
            let ttl = Duration::from_millis(module_config.ttl);
            let name = module_config.name.clone();

            let futures = futures.clone();
            async move {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");
                let expire_time = begin_state.start_time + ttl; 
                let begin_data = begin_state.data.clone();
                let  new_state =  if !begin_state.is_processing && now > expire_time {
                    let fut = tokio::spawn(async move {
                        let f = handler1.handle().await; 
                        match f {
                            Ok(ff) => ff,
                            Err(_) => begin_data
                        }
                    });

                    let ns = Meta {
                        is_processing : true, 
                        start_time : now, 
                        data : begin_state.data.clone()
                    };
                    let mut lock = futures.lock().unwrap();
                    lock.insert(name.clone(), fut);
                    ns

                } else {
                    begin_state
                };

                let fut = {
                    let mut lock = futures.lock().unwrap();
                    if let Some(handle) = lock.remove(&name) {
                        handle
                    } else {
                        tokio::spawn(async { 
                            //let out : HashMap<String, String>  = [].iter().map(|(k,v)|(k.to_string(), v.to_string())).collect();
                            HashMap::new()
                        })
                    }
                };
                let mut fut_opt = Some(fut); 

                let r = fut_opt.as_mut().unwrap().now_or_never();
                let new_new_state = match r {
                    Some(Ok(res)) => {
                        let mut temp = new_state.clone();
                        temp.data.extend(res.clone());
                        Meta {
                            is_processing: false,
                            start_time: new_state.start_time,
                            data: temp.data,
                        }
                    },
                    Some(Err(_)) => {
                        let mut lock = futures.lock().unwrap();
                        lock.insert(name.clone(), fut_opt.take().unwrap()); // now safe
                        new_state.clone()
                    }
                    None => {
                        let mut lock = futures.lock().unwrap();
                        lock.insert(name.clone(), fut_opt.take().unwrap()); // now safe
                        new_state.clone()
                    }
                };

                // let r = fut.now_or_never();
                // let new_new_state = if let Some(Ok(res)) = r {
                //     let mut temp = new_state.clone();
                //     temp.data.extend(res.clone());
                //     let new_meta = Meta {
                //         is_processing: false,
                //         start_time: new_state.start_time, 
                //         data: temp.data
                //     };
                //     new_meta
                //
                //
                // } else {
                //         let mut lock = futures.lock().unwrap();
                //         lock.insert(name.clone(), fut);
                //
                //         new_state.clone()
                //
                // }
                // let new_new_state = match tokio::select! {
                //     res = &mut fut => Some(res),
                //     else => None,
                // } {
                //     Some(Ok(data)) => {
                //         let mut temp = new_state.clone();
                //         temp.data.extend(data.clone());
                //         let new_meta = Meta {
                //             is_processing: false,
                //             start_time: new_state.start_time, 
                //             data: temp.data
                //         };
                //         new_meta
                //     },
                //     Some(Err(_)) => {
                //         let mut lock = futures.lock().unwrap();
                //         lock.insert(name.clone(), fut);
                //
                //         new_state.clone()
                //     },
                //     None => 
                //     {
                //         let mut lock = futures.lock().unwrap();
                //         lock.insert(name.clone(), fut);
                //         new_state.clone()
                //     },
                // };


                let nnsd = new_new_state.clone().data;
                (name.to_string(), new_new_state, handler2.render(nnsd))
            }});

        let values = join_all(futs).await; 

        let out_objs: Vec<types::Out> = values.into_iter().map(|(name, meta, out_str)|{
            state.insert(name.clone(), meta.clone());
            types::Out {
                name: name.clone(),
                instance: name.clone(),
                full_text: out_str.to_string()
            }
        }).collect();

        render_sender.send(out_objs).await?;
        state_sender.send(state.clone()).await?;

        let elapsed = loop_begin.elapsed();
        let wait_time = poll_time_ms.checked_sub(elapsed).unwrap_or(Duration::ZERO);
        tokio::time::sleep(wait_time).await;
    }
}




