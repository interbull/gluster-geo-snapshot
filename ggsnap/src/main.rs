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
//extern crate snapurd_util;
//extern crate snapurd;

mod stats;

use chrono::prelude::*;
use clap::{Arg, ArgMatches, App};
use std::process::Command;



#[derive(Debug)]
struct GeoGluster {
    volume: String,
    slave_vol: String,
    slave_user: String,
    slave_host: String,
}

impl GeoGluster {
    fn new() -> GeoGluster {
        GeoGluster { 
            volume: String::from(""),
            slave_vol: String::from(""),
            slave_user: String::from("geouser"),
            slave_host: String::from("urd-gds-geo-001")
        }
    }
}



fn main() {
    let mut geo_gluster = GeoGluster::new();
    let mut stats = false;
    let matches = arg_matches();
    
    if matches.is_present("VOLUME") &&
       matches.is_present("SLAVE")  &&
       matches.is_present("USER")   &&
       matches.is_present("SLAVE_HOST") {
        geo_gluster.volume = String::from(matches.value_of("VOLUME").unwrap());
        geo_gluster.slave_vol = String::from(matches.value_of("SLAVE").unwrap());
        geo_gluster.slave_user = String::from(matches.value_of("USER").unwrap());
        geo_gluster.slave_host = String::from(matches.value_of("SLAVE_HOST").unwrap());
    }
    else if matches.is_present("VOLUME") &&
            matches.is_present("SLAVE")  &&
            matches.is_present("SLAVE_HOST") {
        geo_gluster.volume = String::from(matches.value_of("VOLUME").unwrap());
        geo_gluster.slave_vol = String::from(matches.value_of("SLAVE").unwrap());
        geo_gluster.slave_host = String::from(matches.value_of("SLAVE_HOST").unwrap());
    }
    else if matches.is_present("VOLUME") &&
            matches.is_present("USER")  &&
            matches.is_present("SLAVE_HOST") {
        geo_gluster.volume = String::from(matches.value_of("VOLUME").unwrap());
        geo_gluster.slave_vol = geo_gluster.volume.clone();
        geo_gluster.slave_user = String::from(matches.value_of("USER").unwrap());
        geo_gluster.slave_host = String::from(matches.value_of("SLAVE_HOST").unwrap());
    }
    else if matches.is_present("VOLUME") &&
            matches.is_present("SLAVE")  &&
            matches.is_present("USER") {
        geo_gluster.volume = String::from(matches.value_of("VOLUME").unwrap());
        geo_gluster.slave_vol = String::from(matches.value_of("SLAVE").unwrap());
        geo_gluster.slave_user = String::from(matches.value_of("USER").unwrap());
    }
    else if matches.is_present("VOLUME") &&
            matches.is_present("SLAVE") {
        geo_gluster.volume = String::from(matches.value_of("VOLUME").unwrap());
        geo_gluster.slave_vol = String::from(matches.value_of("SLAVE").unwrap());
    }
    else if matches.is_present("VOLUME") &&
            matches.is_present("USER") {
        geo_gluster.volume = String::from(matches.value_of("VOLUME").unwrap());
        geo_gluster.slave_vol = geo_gluster.volume.clone();
        geo_gluster.slave_user = String::from(matches.value_of("USER").unwrap());
    }
    else if matches.is_present("VOLUME") &&
            matches.is_present("SLAVE_HOST") {
        geo_gluster.volume = String::from(matches.value_of("VOLUME").unwrap());
        geo_gluster.slave_vol = geo_gluster.volume.clone();
        geo_gluster.slave_host = String::from(matches.value_of("SLAVE_HOST").unwrap());

    }
    else if matches.is_present("VOLUME") {
        geo_gluster.volume = String::from(matches.value_of("VOLUME").unwrap());
        geo_gluster.slave_vol = geo_gluster.volume.clone();
    }
    else {
        let success = print_statistics(&geo_gluster);

        if success.is_err() {
            std::process::exit(1);
        }
        stats = true;
    }

    if !stats {
        let res = create_snapshot(&geo_gluster);

        if res.is_err() {
            std::process::exit(1);
        }
    }
}

fn create_snapshot(geo_gluster: &GeoGluster) -> Result<(), String> {
    let date = Local::now();
    let mut log = String::new();
    log = format!("===================\n{}", date.format("%Y-%m-%d %H:%M:%S"));
    log = format!("{}\nMaster: Pausing geo-replication", log);

    let cmd_out = Command::new("/usr/sbin/gluster")
                          .arg("volume")
                          .arg("geo-replication")
                          .arg(&geo_gluster.volume)
                          .arg(format!("{}@{}::{}", geo_gluster.slave_user, geo_gluster.slave_host, geo_gluster.slave_vol))
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

                match resume_geo_replication(&geo_gluster, &log) {
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
            log = format!("{}\nMaster: Error running command: gluster volume geo-replication {} {}@::{} pause", log, geo_gluster.slave_user, geo_gluster.slave_host, geo_gluster.slave_vol);
            log = format!("{}\nMaster: Error: {}", log, e.to_string());
            print_log(&log, false);
            return Err(String::from("Error"))
        }
    }

    let snap_name = format!("snap_{}_{}", geo_gluster.volume, date.format("%Y%m%d_%H%M%S"));
    log = format!("{}\nMaster: Creating snapshot: {}", log, snap_name);

    let cmd_out = Command::new("/usr/sbin/gluster")
                          .arg("snapshot")
                          .arg("create")
                          .arg(&snap_name)
                          .arg(&geo_gluster.volume)
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
            log = format!("{}\nMaster: Error running command: gluster create snapshot {} {} no-timestamp", log, snap_name, geo_gluster.volume);
            log = format!("{}\nMaster: Error: {}", log, e.to_string());
            print_log(&log, false);
            return Err(String::from("Error"))
        }
    }

    match resume_geo_replication(&geo_gluster, &log) {
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

fn resume_geo_replication(geo_gluster: &GeoGluster, log: &String) -> Result<String, String> {
    let mut l: String = String::new();
    l = format!("{}\nMaster: Resuming geo-replication", log);
    let cmd_out = Command::new("/usr/sbin/gluster")
                          .arg("volume")
                          .arg("geo-replication")
                          .arg(&geo_gluster.volume)
                          .arg(format!("{}@{}::{}", geo_gluster.slave_user, geo_gluster.slave_host, geo_gluster.slave_vol))
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
                        l, geo_gluster.volume, geo_gluster.slave_user, geo_gluster.slave_host, geo_gluster.slave_vol);
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

fn print_statistics(geo_gluster: &GeoGluster) -> Result<(),()>{
    //TODO
    // Impement -i --info flag in main, use with -H --host
    let cmd_out = Command::new("/bin/ssh")
                          .arg(&geo_gluster.slave_host)
                          .arg("/root/ggsnap_slave")
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
                     geo_gluster.slave_host,e);
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

fn arg_matches() -> ArgMatches<'static> {
    App::new("ggsnap")
        .about("Creates snapshots for geo gluster clusters, both on master and slave cluster")
        .version("version 0.1")
        .author("Marcus Pedersén <marcus.pedersen@slu.se>")
        .usage("ggsnap [-suvHi]")
        .arg(Arg::with_name("VOLUME")
            .short("v")
             .long("volume")
             .requires("SLAVE")
            .takes_value(true)
            .help("Takes gluster snapshot on volume VOLUME
both on master and slave cluster
Saves snapshots every day for X days,
saves 2 snapshots for X months back,
saves 1 snapshot per month for X months
Deletes the rest of snapshots
If only this option is given both master
and slave volume has the same name"))
       .arg(Arg::with_name("SLAVE")
            .short("s")
            .long("slave")
            .takes_value(true)
            .requires("VOLUME")
            .help("If slave volume has a different name
this is the name of slave volume.
SLAVE option must be used together with
-v, --volume to specify the name of
master volume"))
       .arg(Arg::with_name("USER")
            .short("u")
            .long("user")
            .takes_value(true)
            .requires("VOLUME")
            .help("Username for geo-replication user,
This option must be used together with VOLUME"))
       .arg(Arg::with_name("SLAVE_HOST")
            .short("H")
            .long("host")
            .takes_value(true)
            .help("Hostname for primary slave node,
This option must be used together with VOLUME"))
       .arg(Arg::with_name("INFO")
            .short("i")
            .long("info")
            .help("Shows statistics on snapshots
for both master and slave cluster.
Use SLAVE_HOST to specify other hostname"))
       .after_help("Important! This program must run on master node")
       .get_matches()
}
