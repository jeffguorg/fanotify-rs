use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::ops::BitOr;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};

use ::fanotify::{consts::*, *};
use clap::Parser;
use log::*;
use nix::sys::signal::{SaFlags, SigAction, SigSet, Signal};

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("fanotify failed: {0}")]
    FanotifyError(#[from] ::fanotify::Error),

    #[error("nix failed: {0}")]
    Errno(#[from] nix::Error),
}

extern "C" fn interrupt_handler(_: i32) {
    info!("exiting");
    std::process::exit(0);
}

#[derive(Debug, clap::Parser)]
#[clap(about="fanotify demo", long_about="
monitor filesystem changes demo

to use as storage provider:
   --providers PROVIDER_COMMAND           specify what command can write to file, like you can simulate it with tee
   --init-flags FAN_CLASS_PRE_CONTENT     this is needed to instruct fanotify send permission events and wait for response
   --mask-flags FAN_CLOSE_WRITE           to know that storage provider has done writing into the file
   --mask-flags FAN_OPEN_PERM             setup permission notification
   --mask-flags FAN_ACCESS_PERM           setup permission notification
   --mask-flags FAN_ON_DIR                create events for directories itself
   --mask-flags FAN_EVENT_ON_CHILD        create events for direct children
")]
struct Args {
    #[clap(required(true))]
    path: Vec<String>,

    #[clap(long, short, default_values_t=default_whitelist())]
    whitelist: Vec<String>,
    #[clap(long, short, default_values_t=default_providers())]
    providers: Vec<String>,

    #[clap(long, short, default_values_t=default_init_flags())]
    init_flags: Vec<String>,
    #[clap(long, short, default_values_t=default_event_f_flags())]
    event_f_flags: Vec<String>,
    #[clap(long, short, default_values_t=default_mask_flags())]
    mask_flags: Vec<String>,
}

fn default_whitelist() -> Vec<String> {
    vec![]
}

fn default_providers() -> Vec<String> {
    vec!["tee".to_string()]
}

fn default_init_flags() -> Vec<String> {
    vec!["FAN_CLASS_NOTIF", "FAN_REPORT_FID"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn default_event_f_flags() -> Vec<String> {
    vec!["O_RDWR", "O_LARGEFILE"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn default_mask_flags() -> Vec<String> {
    vec![
        "FAN_ACCESS",
        "FAN_OPEN",
        "FAN_CLOSE",
        "FAN_ONDIR",
        "FAN_EVENT_ON_CHILD",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

fn reduce_flags<
    S: Into<String>,
    I: IntoIterator<Item = S>,
    F: bitflags::Flags + BitOr<Output = F> + Debug,
>(
    iter: I,
) -> F {
    iter.into_iter()
        .map(|s| {
            let s: String = s.into();
            let Some(flag) = F::from_name(&s.as_str()) else {
                panic!("invalid value for flag: {}", s)
            };
            trace!("flag {} is parted into {:?}", s, flag);

            flag
        })
        .reduce(|a, b| a | b)
        .unwrap_or(F::empty())
}

fn main() -> Result<(), Error> {
    env_logger::Builder::from_default_env().init();

    let args = Args::parse();

    let init_flags: InitFlags = reduce_flags(&args.init_flags);
    let event_f_flags: EventFFlags = reduce_flags(&args.event_f_flags);
    let mask_flags: MaskFlags = reduce_flags(&args.mask_flags);

    info!("init flag: {:x} {:?}", init_flags.bits(), init_flags);
    info!(
        "event fd flag: {:x} {:?}",
        event_f_flags.bits(),
        event_f_flags
    );
    info!("mask flag: {:x} {:?}", mask_flags.bits(), mask_flags);

    let fan = Fanotify::init(init_flags, event_f_flags)?;
    for path in args.path {
        debug!("marking path: {path}");
        fan.mark(MarkFlags::FAN_MARK_ADD, mask_flags, None, Some(&path))?;
        info!("path marked: {path}");
    }
    unsafe {
        nix::sys::signal::sigaction(
            nix::sys::signal::SIGINT,
            &SigAction::new(
                nix::sys::signal::SigHandler::Handler(interrupt_handler),
                SaFlags::SA_RESETHAND,
                SigSet::from(Signal::SIGINT),
            ),
        )?
    };
    info!("interrupt handler is set");

    let whitelist = args.whitelist;
    let storage_provider = args.providers;
 
    let mut ready = HashSet::new();
    let mut bufferdfds: HashMap<std::path::PathBuf, Vec<OwnedFd>> = HashMap::new();
    let mut arg0map = HashMap::new();
    loop {
        let mut events = fan.read_events()?;
        if events.len() == 0 {
            assert!(init_flags & InitFlags::FAN_NONBLOCK == InitFlags::FAN_NONBLOCK);
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
        }
        for event in events.iter_mut() {
            let Some(fd) = event.fd() else {
                warn!("queue full");
                continue;
            };
            let path = match std::fs::read_link(format!("/proc/self/fd/{}", fd.as_raw_fd())) {
                Ok(p) => p,
                Err(err) => {
                    warn!(
                        "failed to read fd link for fd {}: {:?}",
                        fd.as_raw_fd(),
                        err
                    );
                    continue;
                }
            };
            let cmdline_raw = match std::fs::read(format!("/proc/{}/cmdline", event.pid())) {
                Ok(raw) => raw,
                Err(err) => {
                    warn!(
                        "failed to read pid cmdline for fd {}: {:?}",
                        event.pid(),
                        err
                    );
                    continue;
                }
            };

            let cmdline = if cmdline_raw.len() > 0 {
                Some(
                    cmdline_raw
                        .split(|&b| b == 0)
                        .map(|v| String::from_utf8_lossy(v))
                        .collect::<Vec<Cow<str>>>(),
                )
            } else {
                None
            };

            trace!(
                "++++++++= {:?} {} {:?} {:?}",
                fd,
                event.pid(),
                event.mask(),
                path
            );
            let arg0 = if let Some(cmdline) = cmdline.as_ref() {
                for (idx, arg) in cmdline.iter().enumerate() {
                    trace!(" - {}: {}", idx, arg);
                }

                let arg0 = cmdline[0].to_string();

                arg0map.insert(event.pid(), arg0.clone());

                arg0
            } else if arg0map.contains_key(&event.pid()) {
                arg0map[&event.pid()].clone()
            } else {
                "".to_string()
            };
            match event.mask() {
                MaskFlags::FAN_ACCESS_PERM
                | MaskFlags::FAN_OPEN_PERM
                | MaskFlags::FAN_OPEN_EXEC_PERM => {
                    let allowed = match std::fs::metadata(&path) {
                        Ok(metadata) => {
                            // is a directory or filled with content
                            metadata.is_dir() || ready.contains(&path)
                        }
                        Err(error) => {
                            error.kind() == std::io::ErrorKind::NotFound
                        }
                    };
                    if allowed || whitelist.contains(&arg0) || storage_provider.contains(&arg0) {
                        info!("<<<<< {} allowed", fd.as_raw_fd());
                        if let Err(err) =
                            fan.write_response(FanotifyResponse::new(fd, Response::FAN_ALLOW))
                        {
                            warn!("write response for {} failed: {}", fd.as_raw_fd(), err);
                        }
                    } else {
                        let fd = event.forget_fd();
                        info!("<<<<< {} defered", fd.as_raw_fd());
                        if let Some(fds) = bufferdfds.get_mut(&path) {
                            fds.push(fd);
                        } else {
                            bufferdfds.insert(path, vec![fd]);
                        }
                        
                    }
                }
                MaskFlags::FAN_CLOSE_WRITE => {
                    if storage_provider.contains(&arg0) {
                        ready.insert(path.clone());
                        if let Some(fds) = bufferdfds.remove(&path) {
                            for fd in fds {
                                if let Err(err) = fan.write_response(FanotifyResponse::new(
                                    fd.as_fd(),
                                    Response::FAN_ALLOW,
                                )) {
                                    warn!("write response for {} failed: {}", fd.as_raw_fd(), err);
                                }
                                info!(">>>>> {} allowed(defer)", fd.as_raw_fd());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
