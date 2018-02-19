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

use clap::{Arg, ArgMatches, App};
use std::process::Command;

fn main() {
    let matches = arg_matches();

    if matches.is_present("LIST") {
        match list_snapshots() {
            Ok(o) => println!("{}", o),
            Err(e) => {
                println!("{}", e);
                std::process::exit(1);
            }
        }
    }
    else if matches.is_present("VOLUME") {
        println!("Create snapshot");
    }
}

fn list_snapshots() -> Result<String, String> {
    let cmd_out = Command::new("/usr/sbin/gluster")
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

fn arg_matches() -> ArgMatches<'static> {
    App::new("ggsnap_slave")
        .about("Slave program for ggsnap, creates snapshot on gluster geo cluster")
        .version("version 0.1")
        .author("Marcus Pedersén <marcus.pedersen@slu.se>")
        .usage("ggsnap_slave [-lv]")
        .arg(Arg::with_name("LIST")
             .short("l")
             .long("list")
             .conflicts_with("VOLUME")
             .help("Returns names of all snapshots available"))
        .arg(Arg::with_name("VOLUME")
             .short("v")
             .long("volume")
             .takes_value(true)
             .conflicts_with("LIST")
             .help("Takes gluster snapshot on volume VOLUME
on the slave (geo) cluster
Saves snapshots every day for X days,
saves 2 snapshots for X months back,
saves 1 snapshot per month for X months
Deletes the rest of snapshots"))
        .after_help("Important! This program must run on slave (geo) node")
        .get_matches()
}
