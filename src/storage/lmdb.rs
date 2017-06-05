// lmdb.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Externs
// ===========================================================================


extern crate lmdb_rs;


// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports
use std::env;
use std::io;
use std::path::{Path, PathBuf};

// Third-party imports
use self::lmdb_rs::core::*;
use self::lmdb_rs::traits::{FromMdbValue, ToMdbValue};
// use self::lmdb_rs::Environment;

// Local imports
use super::{KeyFileBuilder, KeyFileError, KeyFileResult, KeyFileStore};


// ===========================================================================
// Helpers
// ===========================================================================


fn default_db_path() -> io::Result<PathBuf> {
    let mut dbpath = env::current_dir()?;
    dbpath.push("sec.db");
    Ok(dbpath)
}


// ===========================================================================
// DB Init
// ===========================================================================


pub struct Init {
    maxdb: usize,
    mode: u32,
    pub path: PathBuf
}


impl Init {

    fn new() -> Init {
        Init {
            maxdb: 128,
            mode: 0b111101101 as u32,
            path: default_db_path()
                .expect("Error with db path"),
         }
    }

    // pub fn max_dbs(mut self, maxdbs: usize) -> Self {
    //     self.maxdb = maxdbs;
    //     self
    // }

    // pub fn mode(mut self, val: u32) -> Self {
    //     self.mode = val;
    //     self
    // }

    fn path(&mut self, val: &Path) -> &Self {
        self.path = PathBuf::from(val);
        self
    }

    fn create(&self) -> Environment {
        let env = Environment::new()
            .max_dbs(self.maxdb)
            .open(self.path.as_path(), self.mode)
            .expect("Error opening db file");
        env
    }
}


// ===========================================================================
// KeyFile
// ===========================================================================


pub struct KeyFile {
    pub dbinit: Init,
    dbenv: Environment,
    dbhandle: DbHandle
}


impl KeyFile {

    fn create(env: &Environment, dbname: &str, dbflags: DbFlags) -> MdbResult<DbHandle> {
        let dbhandle = env.get_db(dbname, dbflags);
        let dbhandle = match dbhandle {
            Ok(h) => h,
            Err(_) => env.create_db(dbname, dbflags)?,
        };
        Ok(dbhandle)
    }

    fn dbget<K: ToMdbValue, V: FromMdbValue>(&self, k: &K) -> MdbResult<V> {
        let value;
        let session = self.dbenv.new_transaction()?;
        {
            let db = session.bind(&self.dbhandle);
            value = db.get(k)?;
        }
        Ok(value)
    }

    fn dbset<K: ToMdbValue, V: ToMdbValue>(&self, k: &K, v: &V) -> MdbResult<()> {
        let session = self.dbenv.new_transaction()?;
        {
            let db = session.bind(&self.dbhandle);
            db.set(k, v)?;
        }
        session.commit()
    }
}


impl KeyFileBuilder for KeyFile {

    fn new(name: &str, envpath: Option<&Path>) -> KeyFile {
        let mut init = Init::new();
        let env = match envpath {
            Some(p) => init.path(p).create(),
            None => init.create()
        };

        // Create DB
        let mut dbflags = DbFlags::empty();
        dbflags.insert(DbCreate);
        let dbhandle = KeyFile::create(&env, name, dbflags)
            .expect("Error creating DB");
        KeyFile {
            dbinit: init,
            dbenv: env,
            dbhandle: dbhandle
        }
    }
}


impl KeyFileStore for KeyFile {

    fn get(&self, k: &Vec<u8>) -> KeyFileResult<Vec<u8>> {
        match self.dbget(k) {
            Ok(v) => Ok(v),
            Err(MdbError::NotFound) => Err(KeyFileError::Key(k.clone())),
            _ => Err(KeyFileError::Other)
        }
    }

    fn set(&self, k: &Vec<u8>, file: &Vec<u8>) -> KeyFileResult<()> {
        match self.dbset(k, file) {
            Ok(_) => Ok(()),
            _ => Err(KeyFileError::Other)
        }
    }
    // fn delete(&self, k: &[u8]) -> Result<(), String>;
}


// ===========================================================================
//
// ===========================================================================
