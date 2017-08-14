// main.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Externs
// ===========================================================================

// Stdlib externs

// Third-party externs
extern crate appdirs;

#[macro_use]
extern crate clap;

// Local externs

extern crate safesec;


// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports

use std::fs;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::exit;

// Third-party imports

use clap::{App, Arg};

// Local imports

use safesec::{Config, serve};


// ===========================================================================
// Helpers
// ===========================================================================


pub struct ConfigBuilder {
    name: String,
    db: Option<PathBuf>,
    addr: Option<SocketAddr>,
}


impl ConfigBuilder {
    fn _default_db(appname: &str) -> io::Result<PathBuf>
    {
        // Get user data dir
        let mut dbdir = appdirs::user_data_dir(Some(appname), None, false)
            .map_err(|_| {
                let err = io::Error::new(
                    io::ErrorKind::NotFound,
                    "User home directory not found",
                );
                err
            })?;

        // Add db dir
        dbdir.push("store");

        Ok(dbdir)
    }

    fn _default_addr() -> SocketAddr
    {
        "127.0.0.1:9999".parse().unwrap()
    }

    fn new(appname: &str) -> Self
    {
        Self {
            name: appname.to_string(),
            db: None,
            addr: None,
        }
    }

    pub fn bindaddr(mut self, addr: SocketAddr) -> Self
    {
        self.addr = Some(addr);
        self
    }

    pub fn dbdir(mut self, dbdir: PathBuf) -> Self
    {
        self.db = Some(dbdir);
        self
    }

    pub fn create(self) -> io::Result<Config>
    {
        // Validate db dir
        let db = match self.db {
            None => {
                let dbdir = Self::_default_db(&self.name)?;

                // Create directory if it doesn't exist
                if !dbdir.is_dir() {
                    fs::create_dir_all(dbdir.as_path())?;
                }
                dbdir
            }
            Some(dbdir) => {
                if !dbdir.is_dir() {
                    let errmsg = format!(
                        "DB directory doesn't exist: {}",
                        dbdir.display()
                    );
                    let err = io::Error::new(io::ErrorKind::NotFound, errmsg);
                    return Err(err);
                }
                dbdir
            }
        };

        let addr = match self.addr {
            None => Self::_default_addr(),
            Some(a) => a,
        };
        let name = self.name;

        Ok(Config {
            name: name,
            dbdir: db,
            bindaddr: addr,
        })
    }
}


impl From<Config> for ConfigBuilder {
    fn from(config: Config) -> ConfigBuilder
    {
        ConfigBuilder {
            name: config.name,
            db: Some(config.dbdir),
            addr: Some(config.bindaddr),
        }
    }
}


fn config(appname: &str) -> ConfigBuilder
{
    ConfigBuilder::new(appname)
}


// ===========================================================================
// Main
// ===========================================================================

type AppResult<T> = Result<T, String>;


fn cli() -> AppResult<Config>
{
    let appname = "safesec";
    let default_dbdir = match ConfigBuilder::_default_db(appname) {
        Err(e) => {
            return Err(format!("{}", e));
        }
        Ok(db) => db,
    };
    let default_addr = ConfigBuilder::_default_addr();

    let matches = App::new(appname)
        .version(crate_version!())
        .about("Stores and retrieves binary key/value data")
        .arg(
            Arg::with_name("dbdir")
                .short("d")
                .long("dbdir")
                .value_name("DIR")
                .help(&format!(
                    "Location of db (default: {})",
                    default_dbdir.display()
                ))
                .takes_value(true),
        )
        .arg(
            Arg::with_name("bind_addr")
                .short("b")
                .long("bindaddr")
                .value_name("BINDADDR")
                .help(&format!(
                    "Address and port to bind server to (default: {})",
                    default_addr
                ))
                .takes_value(true),
        )
        .get_matches();

    // Get db value
    let db = matches
        .value_of("dbdir")
        .map(|v| Some(PathBuf::from(v)))
        .unwrap_or(None);

    // Get bindaddr val
    let addr = value_t!(matches, "bind_addr", SocketAddr)
        .map(|v| Some(v))
        .or_else(|e| match e.kind {
            clap::ErrorKind::ArgumentNotFound => Ok(None),
            _ => Err(format!("{}", e)),
        });

    let mut config = config(appname);
    if let Some(db) = db {
        config = config.dbdir(db);
    }

    match addr {
        Ok(None) => {}
        Ok(Some(addr)) => {
            config = config.bindaddr(addr);
        }
        Err(msg) => return Err(msg),
    }

    let config = config.create();
    match config {
        Ok(c) => Ok(c),
        Err(e) => Err(format!("{}", e)),
    }
}


fn main()
{
    let exit_code = {
        let config = match cli() {
            Err(msg) => {
                eprintln!("{}", msg);
                exit(1)
            }
            Ok(c) => c,
        };

        // Start server
        println!("{} running", &config.name);
        if let Err(e) = serve(&config) {
            eprintln!("Server failed: {}", e);
            1
        } else {
            0
        }
    };

    exit(exit_code);
}


// ===========================================================================
//
// ===========================================================================
