// lmdb.rs
// Copyright (C) 2017 authors and contributors (see AUTHORS file)
//
// This file is released under the MIT License.

// ===========================================================================
// Imports
// ===========================================================================


// Stdlib imports

use std::env;
use std::io;
use std::path::{Path, PathBuf};

// Third-party imports

use lmdb::{Database, DatabaseFlags, Environment, Error as LmdbError,
           Result as LmdbResult, Transaction, WriteFlags};
use lmdb_sys::mode_t;

// Local imports

use storage::{KeyFileBuilder, KeyFileError, KeyFileResult, KeyFileStore};


// ===========================================================================
// Helpers
// ===========================================================================


fn default_db_path() -> io::Result<PathBuf>
{
    let mut dbpath = env::current_dir()?;
    dbpath.push("sec.db");
    Ok(dbpath)
}


// ===========================================================================
// DB Init
// ===========================================================================


pub struct Init {
    maxdb: u32,
    mode: mode_t,
    pub path: PathBuf,
}


impl Init {
    fn new() -> Init
    {
        Init {
            maxdb: 128,
            // mode: 0b111101101 as u32,
            mode: 0o600,
            path: default_db_path().expect("Error with db path"),
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

    fn path(&mut self, val: &Path) -> &Self
    {
        self.path = PathBuf::from(val);
        self
    }

    fn create(&self) -> Environment
    {
        Environment::new()
            .set_max_dbs(self.maxdb)
            .open_with_permissions(self.path.as_path(), self.mode)
            .expect("Error opening db file")
    }
}


// ===========================================================================
// KeyFile
// ===========================================================================


pub struct KeyFile {
    pub dbinit: Init,
    env: Environment,
    db: Database,
}


impl KeyFile {
    fn create(env: &Environment, dbname: &str, dbflags: DatabaseFlags)
        -> LmdbResult<Database>
    {
        let db = env.open_db(Some(dbname));
        match db {
            Ok(db) => Ok(db),
            Err(_) => env.create_db(Some(dbname), dbflags),
        }
    }

    fn dbget<K>(&self, key: &K) -> LmdbResult<Vec<u8>>
    where
        K: AsRef<[u8]>,
    {
        let session = self.env.begin_ro_txn()?;
        let value = Vec::from(session.get(self.db.clone(), key)?);
        session.commit()?;
        Ok(value)
    }

    fn dbset<K, V>(&self, key: &K, val: &V, flags: Option<WriteFlags>)
        -> LmdbResult<()>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let flags = match flags {
            None => WriteFlags::empty(),
            Some(f) => f,
        };
        let mut session = self.env.begin_rw_txn()?;
        session.put(self.db.clone(), key, val, flags)?;
        session.commit()?;
        Ok(())
    }
}


impl KeyFileBuilder for KeyFile {
    fn new(name: &str, envpath: Option<&Path>) -> KeyFile
    {
        let mut init = Init::new();
        let env = match envpath {
            Some(p) => init.path(p).create(),
            None => init.create(),
        };

        // Create DB
        let dbflags = DatabaseFlags::empty();
        let db =
            KeyFile::create(&env, name, dbflags).expect("Error creating DB");
        KeyFile {
            dbinit: init,
            env: env,
            db: db,
        }
    }
}


// TODO: handle all LmdbError variants
impl KeyFileStore for KeyFile {
    fn exists<K>(&self, k: &K) -> bool
    where
        K: AsRef<[u8]>,
    {
        match self.dbget(k) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    fn get<K>(&self, k: &K) -> KeyFileResult<Vec<u8>>
    where
        K: AsRef<[u8]>,
    {
        match self.dbget(k) {
            Ok(v) => Ok(v),
            Err(LmdbError::NotFound) => {
                let key = Vec::from(k);
                Err(KeyFileError::Key(key))
            }
            _ => Err(KeyFileError::Other),
        }
    }

    fn set<K, V>(&self, k: &K, file: &V) -> KeyFileResult<()>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        match self.dbset(k, file, None) {
            Ok(_) => Ok(()),
            _ => Err(KeyFileError::Other),
        }
    }
    // fn delete(&self, k: &[u8]) -> Result<(), String>;
}


// ===========================================================================
//
// ===========================================================================
