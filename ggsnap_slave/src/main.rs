///////////////////////////////////////////////////////////////////////////////////////
//                                                                                   //
//    ggsnap_slave, creates and saves snapshots for gluster geo-replicated clutsers. //
//    Copyright (C) 2018  Marcus Pedersén marcus.pedersen@slu.se                     //
//                                                                                   //
//    This program is free software: you can redistribute it and/or modify           //
//    it under the terms of the GNU General Public License as published by           //
//    the Free Software Foundation, either version 3 of the License, or              //
//    (at your option) any later version.                                            //
//                                                                                   //
//    This program is distributed in the hope that it will be useful,                //
//    but WITHOUT ANY WARRANTY; without even the implied warranty of                 //
//    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the                  //
//    GNU General Public License for more details.                                   //
//                                                                                   //
//    You should have received a copy of the GNU General Public License              //
//    along with this program.  If not, see <http://www.gnu.org/licenses/>.          //
//                                                                                   //
///////////////////////////////////////////////////////////////////////////////////////

extern crate clap;
extern crate ggsnap_utils;

use clap::{Arg, ArgMatches, App};
use std::process::Command;
use ggsnap_utils::{get_config, Config, ConfigReadErr };

/// Parses command line arguments and
/// checks that configuration is correct
fn main() {
    let matches = arg_matches();

    if matches.is_present("LIST") || matches.is_present("VOLUME") ||
       matches.is_present("SNAPSHOT_NAME") {
        let mut snapshot_name: String = String::new();
        let mut config_file_exist = true;
        let mut config = Config::default_config();
        config = match get_config() {
            Ok(c) => c,
            Err((e, e_str)) => {
                if e == ConfigReadErr::ConfigNotFound {
                    println!("Slave: {:?}: Config file not found, using default values", e);
                    config_file_exist = false;
                    Config::default_config()
                }
                else {
                    println!("Slave: {:?}: Error reading config file\n{}", e, e_str);
                    std::process::exit(1);
                }
            }
        };

        if config.snapshot.slave_volume.is_none() {
            config.snapshot.slave_volume = config.snapshot.master_volume.clone();
        }

        match matches.value_of("VOLUME") {
            Some(v) => config.snapshot.slave_volume = Some(String::from(v)),
            None    => (),
        }

        let config = config;

        match matches.value_of("SNAPSHOT_NAME") {
            Some(s) => snapshot_name = String::from(s),
            None    => (),
        }

        let snapshot_name = snapshot_name;

        if matches.is_present("LIST") {
            match list_snapshots(&config) {
                Ok(o) => println!("{}", o),
                Err(e) => {
                    println!("{}", e);
                    std::process::exit(1);
                }
            }
        }
        else if matches.is_present("VOLUME") || matches.is_present("SNAPSHOT_NAME") {
            if config.snapshot.slave_volume == None {
                println!("Slave: Error: Missing config value slave volume name: slave_volume");
                if config_file_exist {
                    println!("Slave: Add argument VOLUME or update config file and try again");
                }
                else {
                    println!("Slave: Add argument VOLUME or create a config file and try again");
                }

                println!("");
                println!("Use -h or --help for help");
                std::process::exit(1);
            }
            
            match create_snapshot(&config, &snapshot_name) {
                Ok(l) => println!("{}", l),
                Err(l) => {
                    println!("{}", l);
                    std::process::exit(1);
                },
            }
        }
    }
}

/// Returns the names of all snapshots available
fn list_snapshots(config: &Config) -> Result<String, String> {
    let cmd_out = Command::new(&config.general.gluster_bin)
                          .arg("snapshot")
                          .arg("list")
                          .output();

    match cmd_out {
        Ok(o) => {
            if o.status.success() {
                Ok(String::from_utf8_lossy(&o.stdout).to_string())
            }
            else {
                Err(format!("Slave: {}{}", String::from_utf8_lossy(&o.stdout), 
                            String::from_utf8_lossy(&o.stderr)))
            }
        }
        Err(e) => Err(format!("Slave: Error running command: gluster snapshot list; {}", e)),
    }
}

/// Creates snapshot and returns result
fn create_snapshot(config: &Config, snap_name: &String) -> Result<String, String> {
    let mut log: String = format!("Slave: Creating snapshot: {} on volume: {}", 
                                  snap_name, config.snapshot.slave_volume.clone().unwrap());

    let cmd_out = Command::new(&config.general.gluster_bin)
                          .arg("snapshot")
                          .arg("create")
                          .arg(snap_name)
                          .arg(&config.snapshot.slave_volume.clone().unwrap())
                          .arg("no-timestamp")
                          .output();

    match cmd_out {
        Ok(o) => {
            log = format!("{}\nSlave: {}{}", log, String::from_utf8_lossy(&o.stdout), 
                          String::from_utf8_lossy(&o.stderr));
            if o.status.success() {
                Ok(log)
            }
            else {
                Err(log)
            }
        }
        Err(e) => {
            log = format!("{}\nSlave: Error running command: gluster snapshot create {} {} no-timestamp",
                          log, snap_name, config.snapshot.slave_volume.clone().unwrap());
            log = format!("{}\nSlave: Error: {}", log, e.to_string());
            Err(log)
        }
    }
}

/// Build argument parsing and help text
fn arg_matches() -> ArgMatches<'static> {
    App::new("ggsnap_slave")
        .about("Slave program for ggsnap, creates snapshot on gluster geo cluster")
        .version("version 0.1")
        .author("Marcus Pedersén <marcus.pedersen@slu.se>")
        .usage("ggsnap_slave [OPTION]")
        .arg(Arg::with_name("LIST")
             .short("l")
             .long("list")
             .conflicts_with_all(&["VOLUME", "SNAPSHOT_NAME"])
             .help("Returns names of all snapshots available"))
        .arg(Arg::with_name("VOLUME")
             .short("v")
             .long("volume")
             .takes_value(true)
             .requires("SNAPSHOT_NAME")
             .conflicts_with("LIST")
             .help("Creates gluster snapshot on volume VOLUME
on the slave (geo) cluster
Saves snapshots according to settings
in config file.
Deletes snapshots according to settings in config file.
Requires SNAPSHOT_NAME."))
        .arg(Arg::with_name("SNAPSHOT_NAME")
             .short("n")
             .long("snapshot-name")
             .takes_value(true)
             .conflicts_with("LIST")
             .help("Creates gluster snapshot on slave cluster.
SNAPSHOT_NAME will be the name of the snapshot.
Takes information about VOLUME from config file.
Saves snapshots according to settings
in config file.
Deletes snapshots according to settings in config file."))
        .after_help("Important! This program must run on slave (geo) node

ggsnap_slave is executed from ggsnap that is on main mater node")
        .get_matches()
}
