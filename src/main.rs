use clap::{App, Arg};
use egg_mode;
use serde_derive::{Deserialize, Serialize};

use bimap::BiHashMap;
use tokio_core::reactor::Core;

#[derive(Debug, Clone, Deserialize)]
struct Config {
    ck: String,
    cs: String,
    tk: String,
    ts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    id: Option<u64>,
    sn: Option<String>,
    memo: String,
}

fn main() -> Result<(), Box<std::error::Error>> {
    let mut core = Core::new()?;

    let config = toml::from_str::<Config>(&std::fs::read_to_string("config.toml")?)?;

    let matches = App::new("Twitter Account List")
        .arg(Arg::with_name("path").required(true).index(1))
        .get_matches();

    let token = egg_mode::Token::Access {
        access: egg_mode::KeyPair::new(config.tk, config.ts),
        consumer: egg_mode::KeyPair::new(config.ck, config.cs),
    };

    let path = matches.value_of("path").unwrap();
    let path = format!("data/{}.csv", path);

    let mut records = csv::Reader::from_path(&path)?
        .deserialize::<Record>()
        .collect::<Result<Vec<_>, _>>()?;

    let id_to_sn = core
        .run(egg_mode::user::lookup(
            records
                .iter()
                .flat_map(|Record { id, sn, .. }| match (id, sn) {
                    (Some(id), _) => Some(egg_mode::user::UserID::ID(id.clone())),
                    (None, Some(sn)) => Some(egg_mode::user::UserID::ScreenName(sn)),
                    (None, None) => None,
                })
                .collect::<Vec<_>>(),
            &token,
        ))?
        .response
        .into_iter()
        .map(|user| (user.id, user.screen_name))
        .collect::<BiHashMap<_, _>>();

    for record in records.iter_mut() {
        match (&record.id, &record.sn) {
            (Some(id), _) => {
                if let Some(sn) = id_to_sn.get_by_left(&id) {
                    record.sn = Some(sn.clone());
                }
            }
            (None, Some(sn)) => {
                if let Some(id) = id_to_sn.get_by_right(&sn) {
                    record.id = Some(id.clone());
                }
            }
            (None, None) => {}
        }
    }

    let mut wtr = csv::Writer::from_path(&path)?;

    for record in records {
        wtr.serialize(record)?;
    }

    wtr.flush()?;

    Ok(())
}
