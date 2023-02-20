use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Cursor, Write};
use std::process::exit;
use std::thread;
use std::time::Duration;
use aeonetica_engine::uuid::Uuid;
use aeonetica_engine::libloading::{Library, Symbol};
use aeonetica_engine::error::{AError,AET};
use aeonetica_engine::nanoserde::SerBin;
use aeonetica_engine::{ENGINE_VERSION, Id, log, log_err, MAX_CLIENT_TIMEOUT};
use aeonetica_engine::networking::client_packets::{ClientInfo, ClientMessage, ClientPacket};
use aeonetica_engine::networking::server_packets::{ServerMessage, ServerPacket};
use aeonetica_engine::networking::{MAX_RAW_DATA_SIZE, NetResult};
use aeonetica_engine::util::unzip_archive;
use client::{ClientMod, ClientModBox};
use crate::networking::NetworkClient;

#[cfg(target_os = "windows")]
mod paths_util {
    pub(crate) const MOD_FILE_EXTENSION: &str = ".dll";
    pub(crate) fn client_lib(path: &str, name: &str) -> String {
        &format!("runtime/{path}/{name}_client{MOD_FILE_EXTENSION}")
    }
}

#[cfg(target_os = "linux")]
mod paths_util {
    pub(crate) const MOD_FILE_EXTENSION: &str = ".so";
    pub(crate) fn client_lib(path: &str, name: &str) -> String {
        &format!("runtime/{path}/lib{name}_client{MOD_FILE_EXTENSION}")
    }
}

mod paths_util_common {
    pub(crate) fn mod_hash(path: &str) -> String {
        format!("runtime/{path}.hash")
    }
}

pub(crate) use paths_util::*;
use crate::client_runtime::paths_util_common::mod_hash;

#[derive(Debug, PartialEq)]
pub(crate) enum ClientState {
    Start,
    Registered,
    DownloadedMods,
}

pub(crate) struct ClientRuntime {
    pub(crate) client_id: Id,
    pub(crate) mod_profile: String,
    pub(crate) mod_profile_version: String,
    pub(crate) nc: NetworkClient,
    pub(crate) awaiting_replies: HashMap<Id, Box<dyn Fn(&mut ClientRuntime, &ServerPacket)>>,
    pub(crate) loaded_mods: Vec<ClientModBox>,
    pub(crate) state: ClientState,
    modloading: Modloading
}

pub(crate) struct Modloading{
    mod_list: HashMap<String, (String, String, Vec<String>, String, u64, u64, Vec<u8>, bool)>
}

impl ClientRuntime {
    pub(crate) fn create(client_id: Id, addr: &str, server_addr: &str) -> Result<Self, AError>{
        let nc = NetworkClient::start(addr, server_addr).map_err(|e| {
            e.log_exit();
        }).unwrap();
        log!("started client {addr} and initiating handshake to {server_addr}");
        let mut client = Self {
            client_id,
            nc,
            mod_profile: String::new(),
            mod_profile_version: String::new(),
            awaiting_replies: Default::default(),
            loaded_mods: vec![],
            state: ClientState::Start,
            modloading: Modloading {
                mod_list: Default::default(),
            }
        };
        client.register();
        let timeout_socket = client.nc.socket.try_clone()?;
        thread::spawn(move || {
            loop {
                let data = SerBin::serialize_bin(&ClientPacket {
                    client_id: client_id.clone(),
                    conv_id: Uuid::new_v4().into_bytes(),
                    message: ClientMessage::KeepAlive,
                });
                let _ = timeout_socket.send(data.as_slice()).map_err(|e|{
                    let e: AError = e.into();
                    log_err!("{e}");
                    exit(1);
                });
                std::thread::sleep(Duration::from_millis((MAX_CLIENT_TIMEOUT - 1000) as u64))
            }
        });
        log!("started timeout preventer");
        client.download_mods().map_err(|e| client.gracefully_abort(e));
        client.enable_mods().map_err(|e| client.gracefully_abort(e));
        Ok(client)
    }

    pub(crate) fn request_response<F: Fn(&mut ClientRuntime, &ServerPacket) + 'static>(&mut self, packet: &ClientPacket, handler: F) -> Result<(), AError> {
        self.awaiting_replies.insert(packet.conv_id, Box::new(handler));
        self.nc.send(packet)?;
        Ok(())
    }

    fn register(&mut self) {
        self.request_response(&ClientPacket {
            client_id: self.client_id.clone(),
            conv_id: Uuid::new_v4().into_bytes(),
            message: ClientMessage::Register(ClientInfo {
                client_id: self.client_id,
                client_version: ENGINE_VERSION.to_string(),
            }),
        }, |client, resp| {
            match &resp.message {
                ServerMessage::RegisterResponse(res) => {
                    match res {
                        NetResult::Ok(info) => {
                            log!("successfully connected to server");
                            log!("registered client");
                            client.state = ClientState::Registered;
                            client.mod_profile = info.mod_profile.clone();
                            client.mod_profile_version = info.mod_version.clone();
                            log!("server has mod profile {} v{} with {} mod(s):", client.mod_profile, client.mod_profile_version, info.mods.len());
                            client.modloading.mod_list = info.mods.clone().into_iter()
                                .map(|(name_path, flags, hash, size)| {
                                    let (name, path) = name_path.split_once(":").unwrap();
                                    let mut local_hash = String::new();
                                    let _ = File::open(mod_hash(path)).map(|mut f| f.read_to_string(&mut local_hash));
                                    let available = local_hash.trim() == &hash;
                                    log!("  - {name_path}");
                                    if !available {
                                        let _ = std::fs::remove_dir_all(&format!("runtime/{path}"));
                                    }
                                    (name_path.clone(), (name.to_string(), path.to_string(), flags, hash, size, 0, vec![0;size as usize], available))
                                }).collect();
                        }
                        NetResult::Err(msg) => {
                            log_err!("server did not accept connection: {msg}");
                            exit(1);
                        }
                    }
                },
                e => {
                    log_err!("invalid response: {e:?}");
                    exit(1);
                }
            }
        }).map_err(|e| {
            e.log_exit();
        }).unwrap();
        while self.state != ClientState::Registered {
            for packet in self.nc.queued_packets() {
                self.handle_packet(&packet).map_err(|e| {
                    e.log_exit();
                }).unwrap();
            }
        }
    }

    fn download_mods(&mut self) -> Result<(), AError>{
        log!("downloading {} mod(s)", self.modloading.mod_list.values().into_iter().filter(|m| !m.7).count());
        let keys = self.modloading.mod_list.keys().map(|s| s.to_string()).collect::<Vec<_>>();
        let mut total = 0;
        for name_path in keys {
            let d = self.modloading.mod_list.get(&name_path).unwrap();
            if d.7 {
                continue
            }
            let size = d.4;
            log!("downloading mod {name_path} across {} packets", size.div_ceil(MAX_RAW_DATA_SIZE as u64));
            total += size;
            for i in (0..size).step_by(MAX_RAW_DATA_SIZE) {
                let np = name_path.clone();
                self.request_response(&ClientPacket {
                    client_id: self.client_id.clone(),
                    conv_id: Uuid::new_v4().into_bytes(),
                    message: ClientMessage::DownloadMod(name_path.clone(), i),
                }, move |client, resp| {
                    match &resp.message {
                        ServerMessage::RawData(data) => {
                            let m = client.modloading.mod_list.get_mut(&np).unwrap();
                            m.5 += data.len() as u64;
                            m.6.splice(i as usize..(i as usize+data.len()), data.to_owned());
                        },
                        e => {
                            log_err!("invalid response: {e:?}");
                            exit(1);
                        }
                    }
                }).map_err(|e| {
                    e.log_exit();
                }).unwrap();
            }
        }
        let mut p = 0.0;
        while self.state != ClientState::DownloadedMods {
            for packet in self.nc.queued_packets() {
                self.handle_packet(&packet)?;
            }

            let mut downloaded = 0;
            for (key, m) in self.modloading.mod_list.iter_mut(){
                if !m.7 {
                    downloaded += m.5;
                    if m.4 == m.5 {
                        m.7 = true;
                        unzip_archive(Cursor::new(&m.6), &format!("runtime/{}", m.1))?;
                        File::create(&format!("runtime/{}.hash", m.1)).unwrap().write_all(m.3.as_bytes())?;
                        log!("finished downloading mod {}", key)
                    }
                }
            }
            if downloaded as f32 / total as f32 - p > 0.2 {
                p = downloaded as f32 / total as f32;
                log!("progress: {p}")
            }
            if downloaded == total {
                self.state = ClientState::DownloadedMods
            }
        }
        log!("downloaded all missing mods");
        Ok(())
    }

    fn enable_mods(&mut self) -> Result<(), AError>{
        for (name_path, m) in &self.modloading.mod_list {
            log!("loading mod {} ...", name_path);
            let mut loaded_mod = load_mod(name_path)?;
            loaded_mod.init(&m.2);
            self.loaded_mods.push(loaded_mod);
            log!("loaded mod {} ...", name_path);
        }
        log!("successfully loaded {} mods from profile {} v{}", self.loaded_mods.len(), self.mod_profile, self.mod_profile_version);
        Ok(())
    }

    fn gracefully_abort<E: Into<AError>>(&self, e: E) -> !{
        let err = e.into();
        err.log();
        let _ = self.nc.send(&ClientPacket {
            client_id: self.client_id,
            conv_id: Uuid::new_v4().into_bytes(),
            message: ClientMessage::Unregister,
        });
        log_err!("gracefully aborted client");
        exit(1);
    }
}

pub(crate) fn load_mod(name_path: &str) -> Result<ClientModBox, AError> {
    let (name, path) = name_path.split_once(":").unwrap();
    let client_lib = unsafe { Library::new(client_lib(path, name))
        .map_err(|e| AError::new(AET::ModError(format!("could not load mod: {e}"))))? };
    let _create_mod_client: Symbol<fn() -> Box<dyn ClientMod>> = unsafe { client_lib.get("_create_mod_client".as_ref())
        .map_err(|e| AError::new(AET::ModError(format!("could not load mod create function: {e}"))))? };
    let mod_client = _create_mod_client();
    Ok(ClientModBox::new(mod_client, client_lib))
}
