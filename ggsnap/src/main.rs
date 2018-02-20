/////////////////////////////////////////////////////////////////////////////////
//                                                                             //
//    ggsnap, creates and saves snapshots for gluster geo-replicated clutsers. //
//    Copyright (C) 2018  Marcus Pedersén marcus.pedersen@slu.se               //
//                                                                             //
//    This program is free software: you can redistribute it and/or modify     //
//    it under the terms of the GNU General Public License as published by     //
//    the Free Software Foundation, either version 3 of the License, or        //
//    (at your option) any later version.                                      //
//                                                                             //
//    This program is distributed in the hope that it will be useful,          //
//    but WITHOUT ANY WARRANTY; without even the implied warranty of           //
//    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the            //
//    GNU General Public License for more details.                             //
//                                                                             //
//    You should have received a copy of the GNU General Public License        //
//    along with this program.  If not, see <http://www.gnu.org/licenses/>.    //
//                                                                             //
/////////////////////////////////////////////////////////////////////////////////

extern crate chrono;
extern crate clap;
extern crate ggsnap_utils;

mod stats;

use chrono::prelude::*;
use clap::{Arg, ArgMatches, App};
use std::process::Command;
use ggsnap_utils::{get_config, Config, ConfigReadErr };

/// Parses command line arguments and
/// checks that configuration is correct
fn main() {
    let matches = arg_matches();
    let mut config: Config = Config::default_config();
    
    if matches.is_present("VOLUME")   || matches.is_present("SLAVE") ||
       matches.is_present("USER")     || matches.is_present("SLAVE_HOST") ||
       matches.is_present("SNAPSHOT") || matches.is_present("INFO") {

        let mut config_file_exist = true;
        let mut config_err = false;
        config = match get_config() {
            Ok(c) => c,
            Err((e, e_str)) => {
                if e == ConfigReadErr::ConfigNotFound {
                    println!("{:?}: Config file not found, using default values", e);
                    config_file_exist = false;
                    Config::default_config()
                }
                else {
                    println!("{:?}: Error reading config file\n{}", e, e_str);
                    std::process::exit(1);
                }
            },
        };

        match matches.value_of("VOLUME") {
            Some(v) => config.snapshot.master_volume = Some(String::from(v)),
            None => (),
        }

        match matches.value_of("SLAVE") {
            Some(v) => config.snapshot.slave_volume = Some(String::from(v)),
            None => (),
        }

        match matches.value_of("USER") {
            Some(v) => config.snapshot.slave_user = Some(String::from(v)),
            None => (),
        }

        match matches.value_of("SLAVE_HOST") {
            Some(v) => config.snapshot.slave_hostname = Some(String::from(v)),
            None => (),
        }

        if config.snapshot.slave_volume.is_none() {
            config.snapshot.slave_volume = config.snapshot.master_volume.clone();
        }

        if config.snapshot.master_volume.is_none() {
            println!("Error: Missing config value master volume name: master_volume");
            config_err = true;
        }

        if config.snapshot.slave_volume.is_none() {
            println!("Error: Missing config value slave volume name: slave_volume");
            config_err = true;
        }

        if config.snapshot.slave_user.is_none() {
            println!("Error: Missing config value slave user name: slave_user");
            config_err = true;
        }

        if config.snapshot.slave_hostname.is_none() {
            println!("Error: Missing config value slave hostname name: slave_hostname");
            config_err = true;
        }

        if config_err {
            if config_file_exist {
                println!("Add missing arguments or update config file and try again");
            }
            else {
                println!("Add missing arguments or create a config file and try again");
            }

            println!("");
            println!("Use -h or --help for help");
            std::process::exit(1);
        }
    }

    let config = config;

    if matches.is_present("INFO") {
        let success = print_statistics(&config);

        if success.is_err() {
            std::process::exit(1);
        }
    }
    else {
        let res = create_snapshot(&config);

        if res.is_err() {
            std::process::exit(1);
        }
    }
}

/// Pause geo-replication, if already paused it will continue.
/// Creates snapshot on both master and slave node.
/// Resumes geo-replication
fn create_snapshot(config: &Config) -> Result<(), String> {
    let date = Local::now();
    let mut log = String::new();
    log = format!("===================\n{}", date.format("%Y-%m-%d %H:%M:%S"));
    log = format!("{}\nMaster: Pausing geo-replication", log);

    let cmd_out = Command::new(&config.general.gluster_bin)
                          .arg("volume")
                          .arg("geo-replication")
                          .arg(&config.snapshot.master_volume.clone().unwrap())
                          .arg(format!("{}@{}::{}", config.snapshot.slave_user.clone().unwrap(),
                                       config.snapshot.slave_hostname.clone().unwrap(),
                                       config.snapshot.slave_volume.clone().unwrap()))
                          .arg("pause")
                          .output();
    match cmd_out {
        Ok(o) => {
            let o_str = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
            if !o.status.success() && o_str.contains("already Paused") {
                log = format!("{}\nMaster: {}\nMaster: Continue as geo-replication is already paused", log, o_str);
            }
            else if !o.status.success() {
                log = format!("{}\nMaster: {}", log, o_str);

                match resume_geo_replication(&config, &log) {
                    Ok(l) => {
                        log = l;
                    }
                    Err(l) => {
                        log = l;
                    }
                }

                print_log(&log, false);
                return Err(String::from("Error"))
            }
            else {
                log = format!("{}\nMaster: {}", log, o_str);
            }
        }
        Err(e) => {
            log = format!("{}\nMaster: Error running command: gluster volume geo-replication {} {}@::{} pause",
                          log, config.snapshot.slave_user.clone().unwrap(),
                          config.snapshot.slave_hostname.clone().unwrap(),
                          config.snapshot.slave_volume.clone().unwrap());
            log = format!("{}\nMaster: Error: {}", log, e.to_string());
            print_log(&log, false);
            return Err(String::from("Error"))
        }
    }

    let snap_name = format!("snap_{}_{}", config.snapshot.master_volume.clone().unwrap(), date.format("%Y%m%d_%H%M%S"));
    log = format!("{}\nMaster: Creating snapshot: {}", log, snap_name);

    let cmd_out = Command::new(&config.general.gluster_bin)
                          .arg("snapshot")
                          .arg("create")
                          .arg(&snap_name)
                          .arg(&config.snapshot.master_volume.clone().unwrap())
                          .arg("no-timestamp")
                          .output();

    match cmd_out {
        Ok(o) => {
            log = format!("{}\nMaster: {}{}", log, String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
            if !o.status.success() {
                print_log(&log, false);
                return Err(String::from("Error"))
            }
        }
        Err(e) => {
            log = format!("{}\nMaster: Error running command: gluster create snapshot {} {} no-timestamp",
                          log, snap_name, config.snapshot.master_volume.clone().unwrap());
            log = format!("{}\nMaster: Error: {}", log, e.to_string());
            print_log(&log, false);
            return Err(String::from("Error"))
        }
    }

    match resume_geo_replication(&config, &log) {
        Ok(l) => {
            print_log(&l, true);
        }
        Err(l) => {
            print_log(&l, false);
            return Err(String::from("Error"))
        }
    }

    remove_old_snapshots();

    Ok(())
}

/// Resuming of geo-replication
fn resume_geo_replication(config: &Config, log: &String) -> Result<String, String> {
    let mut l: String = String::new();
    l = format!("{}\nMaster: Resuming geo-replication", log);
    let cmd_out = Command::new(&config.general.gluster_bin)
                          .arg("volume")
                          .arg("geo-replication")
                          .arg(&config.snapshot.master_volume.clone().unwrap())
                          .arg(format!("{}@{}::{}", config.snapshot.slave_user.clone().unwrap(),
                                       config.snapshot.slave_hostname.clone().unwrap(),
                                       config.snapshot.slave_volume.clone().unwrap()))
                          .arg("resume")
                          .output();

    match cmd_out {
        Ok(o) => {
            l = format!("{}\nMaster: {}{}", l, String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
            if !o.status.success() {
                return Err(l)
            }
        }
        Err(e) => {
            l = format!("{}\nMaster: Error running command: gluster volume geo-replication {} {}@{}::{} resume", 
                        l, config.snapshot.master_volume.clone().unwrap(),
                        config.snapshot.slave_user.clone().unwrap(),
                        config.snapshot.slave_hostname.clone().unwrap(),
                        config.snapshot.slave_volume.clone().unwrap());
            l = format!("{}\nMaster: Error:{}", l, e.to_string());
            return Err(l)
        }
    }
 
    Ok(l)
}

fn create_slave_snapshot() {
    //TODO
}

fn remove_old_snapshots() {
    //TODO
    println!("Removing old snapshots");
}

fn remove_old_slave_snapshots() {
    //TODO
}

/// Print statistics for both master snapshots
/// and slave snapshots.
/// Prints number of snapshots that differs between
/// master and slave.
/// Only snapshots with names that matches the
/// format ggsnap uses.
fn print_statistics(config: &Config) -> Result<(),()>{
    let cmd_out = Command::new("/bin/ssh")
                          .arg(&config.snapshot.slave_hostname.clone().unwrap())
                          .arg(&config.general.ggsnap_slave_bin)
                          .arg("--list")
                          .output();

    let mut slave_gluster_out: String = String::from("");
    match cmd_out {
        Ok(o) => {
            if o.status.success() {
                slave_gluster_out = String::from_utf8_lossy(&o.stdout).to_string();
            }
            else {
                println!("Master: Error running ggsnap_slave: {}{}", 
                         String::from_utf8_lossy(&o.stdout), 
                         String::from_utf8_lossy(&o.stderr));
                return Err(())
            }
        },
        Err(e) => {
            println!("Master: Error running command: ssh {} /root/ggsnap_slave --list; {}", 
                     config.snapshot.slave_hostname.clone().unwrap(),e);
            return Err(())
        },
    }

    
    let slave_stats = stats::SnapStat::new(slave_gluster_out);
    let stats = stats::get_statistics();

    println!("==================================================================================");
    println!("=               Snapshot statistics (Snapshots created by snapurd)               =");
    println!("==================================================================================");
    println!("Total number of snapshots on master cluster: {}", stats.len());
    println!("Newest snapshot on master cluster: {}", stats.newest_snap());
    println!("Oldest snapshot on master cluster: {}", stats.oldest_snap());
    println!("");
    println!("Total number of snapshots on slave cluster: {}", slave_stats.len());
    println!("Newest snapshot on slave cluster: {}", slave_stats.newest_snap());
    println!("Oldest snapshot on slave cluster: {}", slave_stats.oldest_snap());
    println!("");
    println!("Number of snapshots that differ between master and slave: {}", 
             stats.number_diff(&slave_stats));
    println!("==================================================================================");

    Ok(())
}

fn print_log(log: &String, success: bool) {
    //TODO
    println!("{}\nSuccess: {}", log, success);
}

/// Build argument parsing and help text
fn arg_matches() -> ArgMatches<'static> {
    App::new("ggsnap")
        .about("Creates snapshots for geo gluster clusters, both on master and slave cluster")
        .version("version 0.1")
        .author("Marcus Pedersén <marcus.pedersen@slu.se>")
        .usage("ggsnap [OPTION]")
        .arg(Arg::with_name("VOLUME")
             .short("v")
             .long("volume")
             .conflicts_with_all(&["SNAPSHOT", "INFO"]) 
             .takes_value(true)
             .help("Takes gluster snapshot on volume VOLUME
both on master and slave cluster
Saves snapshots according to settings
in config file.
Deletes snapshots according to settings in config file.
If not SLAVE option is given both master
and slave volume has the same name.
Option USER and SLAVE_HOST is required."))
       .arg(Arg::with_name("SLAVE")
            .short("s")
            .long("slave")
            .conflicts_with_all(&["SNAPSHOT", "INFO"])
            .takes_value(true)
            .help("If slave volume has a different name
this is the name of slave volume.
Options VOLUME, USER and SLAVE_HOST is required
if not specified in config file."))
       .arg(Arg::with_name("USER")
            .short("u")
            .long("user")
            .conflicts_with_all(&["SNAPSHOT", "INFO"])
            .takes_value(true)
            .help("Username for geo-replication user,
Options VOLUME and SLAVE_HOST is required
if not specified in config file."))
       .arg(Arg::with_name("SLAVE_HOST")
            .short("H")
            .long("host")
            .conflicts_with_all(&["SNAPSHOT", "INFO"])
            .takes_value(true)
            .help("Hostname for primary slave node.
Options VOULME and USER is reqiured
if not specified in config file."))
       .arg(Arg::with_name("INFO")
            .short("i")
            .long("info")
            .conflicts_with_all(&["VOLUME", "USER", "SLAVE_HOST", "SLAVE", "SNAPSHOT"])
            .help("Shows statistics on snapshots
for both master and slave cluster.
Option SLAVE_HOST is required
if not specified in config file."))
       .arg(Arg::with_name("SNAPSHOT")
            .short("c")
            .long("create-snapshots")
            .conflicts_with_all(&["VOLUME", "USER", "SLAVE_HOST", "SLAVE", "INFO"])
            .help("Creates snapshots on both
master and slave cluster.
Takes information about SLAVE_HOST, VOLUME,
USER and SLAVE from config file."))
       .after_help("Important! This program must run on master node

To create snapshots two alternatives are available:
use SNAPSHOT flag to use information from config file
use VOLUME, USER, SLAVE_HOST and/or SLAVE options to
override values from config file.
")
       .get_matches()
}
