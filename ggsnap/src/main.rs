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
extern crate lettre;

mod stats;

use chrono::prelude::*;
use clap::{Arg, ArgMatches, App};
use std::process::Command;
use ggsnap_utils::{get_config, Config, ConfigReadErr };
use std::path::Path;
use std::fs::OpenOptions;
use std::io::{ Write, BufWriter };
use lettre::smtp::authentication::{ Credentials, Mechanism };
use lettre::{SimpleSendableEmail, EmailTransport, EmailAddress, SmtpTransport};

/// Parses command line arguments and
/// checks that configuration is correct
fn main() {
    let matches = arg_matches();
    
    if matches.is_present("VOLUME")   || matches.is_present("SLAVE") ||
       matches.is_present("USER")     || matches.is_present("SLAVE_HOST") ||
       matches.is_present("SNAPSHOT") || matches.is_present("INFO") {

        let mut _config: Config = Config::default_config();
        let mut config_file_exist = true;
        let mut config_err = false;
        let mut config_err_text: String = String::new();
        _config = match get_config() {
            Ok(c) => c,
            Err((e, e_str)) => {
                if e == ConfigReadErr::ConfigNotFound {
                    println!("Master: {:?}: Config file not found, using default values", e);
                    config_file_exist = false;
                    Config::default_config()
                }
                else if e == ConfigReadErr::ConfigValueErr {
                    println!("Master: {:?}: Parameter error in config file\n{}", e, e_str);
                    std::process::exit(1);
                }
                else {
                    println!("Master: {:?}: Error reading config file\n{}", e, e_str);
                    std::process::exit(1);
                }
            },
        };

        match matches.value_of("VOLUME") {
            Some(v) => _config.snapshot.master_volume = Some(String::from(v)),
            None => (),
        }

        match matches.value_of("SLAVE") {
            Some(v) => _config.snapshot.slave_volume = Some(String::from(v)),
            None => (),
        }

        match matches.value_of("USER") {
            Some(v) => _config.snapshot.slave_user = Some(String::from(v)),
            None => (),
        }

        match matches.value_of("SLAVE_HOST") {
            Some(v) => _config.snapshot.slave_hostname = Some(String::from(v)),
            None => (),
        }

        if _config.snapshot.slave_volume.is_none() {
            _config.snapshot.slave_volume = _config.snapshot.master_volume.clone();
        }

        if _config.snapshot.snapshot_name_prefix.is_none() {
            let c = Config::default_config();
            _config.snapshot.snapshot_name_prefix = c.snapshot.snapshot_name_prefix.clone();
        }

        if _config.snapshot.master_volume.is_none() {
            config_err_text = String::from("Error: Missing config value master volume name: master_volume");
            config_err = true;
        }

        if _config.snapshot.slave_volume.is_none() {
            if config_err_text.len() == 0 {
                config_err_text = String::from("Error: Missing config value slave volume name: slave_volume");
            }
            else {
                config_err_text = format!("{}\nError: Missing config value slave volume name: slave_volume", config_err_text);
            }
            config_err = true;
        }

        if _config.snapshot.slave_user.is_none() {
            if config_err_text.len() == 0 {
                config_err_text = String::from("Error: Missing config value slave user name: slave_user");
            }
            else {
                config_err_text = format!("{}\nError: Missing config value slave user name: slave_user", config_err_text);
            }
            config_err = true;
        }

        if _config.snapshot.slave_hostname.is_none() {
            if config_err_text.len() == 0 {
                config_err_text = String::from("Error: Missing config value slave hostname name: slave_hostname");
            }
            else {
                config_err_text = format!("{}\nError: Missing config value slave hostname name: slave_hostname", config_err_text);
            }
            config_err = true;
        }

        let config = _config;

        if matches.is_present("INFO") && 
           config.snapshot.slave_hostname.is_some() {
            let success = print_statistics(&config);

            if success.is_err() {
                std::process::exit(1);
            }
        }
        else if config_err {
            println!("{}", config_err_text);
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



        if !matches.is_present("INFO") {
            let res = create_snapshot(&config);

            if res.is_err() {
                std::process::exit(1);
            }
        }
    }
}

/// Pause geo-replication, if already paused it will continue.
/// Creates snapshot on both master and slave node.
/// Resumes geo-replication
fn create_snapshot(config: &Config) -> Result<(), String> {
    let date = Local::now();
    let mut log = String::new();
//    log = format!("===================\n{}", date.format("%Y-%m-%d %H:%M:%S"));
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

                print_log(&log, date, &config, false);
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
            print_log(&log, date, &config, false);
            return Err(String::from("Error"))
        }
    }

    let snap_name = format!("{}_{}_{}", config.snapshot.snapshot_name_prefix.clone().unwrap(),
                            config.snapshot.master_volume.clone().unwrap(), date.format("%Y%m%d_%H%M%S"));

    let mut slave_snap_success = true;
    match create_slave_snapshot(&config, &snap_name) {
        Ok(m) => log = format!("{}\n{}", log, m),
        Err(e) => {
            log = format!("{}\n{}", log, e);
            slave_snap_success = false;
        }
    }

    let slave_snap_success = slave_snap_success;

    log = format!("{}\nMaster: Creating snapshot: {} on volume: {}", 
                  log, snap_name, config.snapshot.master_volume.clone().unwrap());

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
                match resume_geo_replication(&config, &log) {
                    Ok(l) => {
                        print_log(&l, date, &config, false);
                    }
                    Err(l) => {
                        print_log(&l, date, &config, false);
                    }
                }

                return Err(String::from("Error"))
            }
        }
        Err(e) => {
            log = format!("{}\nMaster: Error running command: gluster snapshot create {} {} no-timestamp",
                          log, snap_name, config.snapshot.master_volume.clone().unwrap());
            log = format!("{}\nMaster: Error: {}", log, e.to_string());
            print_log(&log, date, &config, false);
            return Err(String::from("Error"))
        }
    }

    let mut old_snap_success = true;
    match remove_old_snapshots(&config) {
        Ok(s) => log = format!("{}\n{}", log, s),
        Err(e) => {
            log = format!("{}\n{}", log, e);
            old_snap_success = false;
        }
    }

    match remove_old_slave_snapshots(&config) {
        Ok(s) => log = format!("{}\n{}", log, s),
        Err(e) => {
            log = format!("{}\n{}", log, e);
            old_snap_success = false;
        }
    }

    match resume_geo_replication(&config, &log) {
        Ok(l) => {
            print_log(&l, date, &config, slave_snap_success && old_snap_success);
        }
        Err(l) => {
            print_log(&l, date, &config, false);
            return Err(String::from("Error"))
        }
    }



    Ok(())
}

/// Resuming of geo-replication
fn resume_geo_replication(config: &Config, log: &String) -> Result<String, String> {
    let mut _l: String = String::new();
    _l = format!("{}\nMaster: Resuming geo-replication", log);
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
            _l = format!("{}\nMaster: {}{}", _l, String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
            if !o.status.success() {
                return Err(_l)
            }
        }
        Err(e) => {
            _l = format!("{}\nMaster: Error running command: gluster volume geo-replication {} {}@{}::{} resume", 
                        _l, config.snapshot.master_volume.clone().unwrap(),
                        config.snapshot.slave_user.clone().unwrap(),
                        config.snapshot.slave_hostname.clone().unwrap(),
                        config.snapshot.slave_volume.clone().unwrap());
            _l = format!("{}\nMaster: Error:{}", _l, e.to_string());
            return Err(_l)
        }
    }
 
    Ok(_l)
}

/// Connects to main slave node over ssh
/// and runs ggsnap_slave to create a
/// snapshot
fn create_slave_snapshot(config: &Config, snap_name: &String) -> Result<String, String>{
    let cmd_out = Command::new("ssh")
                          .arg(&config.snapshot.slave_hostname.clone().unwrap())
                          .arg(&config.general.ggsnap_slave_bin)
                          .arg("--volume")
                          .arg(&config.snapshot.slave_volume.clone().unwrap())
                          .arg("--snapshot-name")
                          .arg(snap_name)
                          .output();

    match cmd_out {
        Ok(o) => {
            let l = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
            if o.status.success() {
                Ok(l)
            }
            else {
                Err(format!("Slave error creating snapshot:\n{}", l))
            }
        },
        Err(e) => {
            let mut l: String = format!("Master: Error running command: ssh {} {} --volume {} --snapshot-name {}",
                                         config.snapshot.slave_hostname.clone().unwrap(),
                                         config.general.ggsnap_slave_bin,
                                         config.snapshot.slave_volume.clone().unwrap(),
                                         snap_name);
            l = format!("{}\nMaster: Error: {}", l, e.to_string());
            Err(l)
        }           
    }
                          
}

/// Removes old snapshot from master node according 
/// to settings in config file.
fn remove_old_snapshots(config: &Config) -> Result<String, String> {
    let mut log = String::from("Master: Removing old snapshots");

    match ggsnap_utils::remove_old_snapshots(config, ggsnap_utils::HostType::Master) {
        Ok(s) => {
            log = format!("{}\nMaster: The following snapshots has been removed:\n{}", log, s);
            Ok(format!("{}\nMaster: End of removing snapshots", log))
        },
        Err(e) => Err(format!("{}\nMaster: Error removing old snapshots:\n{}", log, e.to_string())),
    }
}

/// Removes old snapshot from slave node according 
/// to settings in config file.
fn remove_old_slave_snapshots(config: &Config) -> Result<String, String> {
    let cmd_out = Command::new("/bin/ssh")
                          .arg(&config.snapshot.slave_hostname.clone().unwrap())
                          .arg(&config.general.ggsnap_slave_bin)
                          .arg("--remove-snapshots")
                          .arg("--volume")
                          .arg(&config.snapshot.slave_volume.clone().unwrap())
                          .output();

    match cmd_out {
        Ok(o) => {
            if o.status.success() {
                Ok(format!("{}", String::from_utf8_lossy(&o.stdout)))
            }
            else {
                Err(format!("Slave: Error running command: {} --remove-snapshots --volume {}\n{}{}",
                            config.general.ggsnap_slave_bin,
                            config.snapshot.slave_volume.clone().unwrap(),
                            String::from_utf8_lossy(&o.stdout), 
                            String::from_utf8_lossy(&o.stderr)))
            }
        },
        Err(e) => {
            Err(format!("Master: Error running command: ssh {} {} --remove-snapshots --volume {}\n{}",
                        config.snapshot.slave_hostname.clone().unwrap(), 
                        config.general.ggsnap_slave_bin, 
                        config.snapshot.slave_volume.clone().unwrap(), e.to_string()))
        }
    }
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

    let mut _slave_gluster_out: String = String::from("");
    match cmd_out {
        Ok(o) => {
            if o.status.success() {
                _slave_gluster_out = String::from_utf8_lossy(&o.stdout).to_string();
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

    
    let slave_stats = stats::SnapStat::new(_slave_gluster_out, &config.snapshot.slave_volume.clone().unwrap());
    let stats = stats::get_statistics(&config);

    println!("==================================================================================");
    println!("=               Snapshot statistics (Snapshots created by ggsnap)                =");
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

/// Prints result to log file in specified
/// log file in config file
/// If mail is active, mail will be sent
/// with result.
fn print_log(log: &String, date: DateTime<Local>, config: &Config, success: bool) {
    //TODO
    // Send mail and and append to log in same dir as ggsnap
    let mut header: String = format!("\n\n====================================================================================\n");
    header = format!("{}=                 gluster-geo-snapshot date: {}                   =",
                     header, date.format("%Y-%m-%d %H:%M:%S"));
    header = format!("{}\n====================================================================================\n", header);

    let mut tail: String = format!("\n====================================================================================\n");

    if success {
        tail = format!("{}=                      Snapshots updated successfully                              =\n", tail);
    }
    else {
        tail = format!("{}=                    -**- Error updating snapshots -**-                            =\n", tail);
    }
    tail = format!("{}====================================================================================\n", tail);

    let mut _log_msg: String = String::new();
    let mut _log_msg_no_tail: String = String::new();
    let mut clean_log: Vec<String> = Vec::new();
    let mut prev_l = "";

    for l in log.split("\n") {
        if l.len() > 0 {
            if l.contains("Do you still want to continue? (y/n)") {
                let s: Vec<&str> = l.split("Do you still want to continue? (y/n)").collect();
                clean_log.push(s[0].to_string());

                if prev_l.starts_with("Master") {
                    clean_log.push(format!("Master: {}", s[1]));
                }
                else {
                    clean_log.push(format!("Slave: {}", s[1]));
                }
            }
            else {
                if l.starts_with("Master") || l.starts_with("Slave") {
                    clean_log.push(l.to_string());
                }
                else {
                    if prev_l.starts_with("Master") {
                        clean_log.push(format!("Master: {}", l));
                    }
                    else {
                        clean_log.push(format!("Slave: {}", l));
                    }
                }
            }
            prev_l = l;
        }
    }

    _log_msg = format!("{}", header);
    for l in clean_log {
        _log_msg = format!("{}\n{}", _log_msg, l);
    }
    _log_msg_no_tail = _log_msg.clone();
    _log_msg = format!("{}\n{}", _log_msg, tail);

    let res_mail = send_log_mail(&_log_msg, &config, success);
    _log_msg = format!("{}\n{}\n{}", _log_msg_no_tail, res_mail, tail);

    match write_to_log_file(&_log_msg, &config) {
        Ok(_) => (),
        Err(e) => println!("{}", e),
    }


}

/// Write log to log file 
/// as specified in config file 
fn write_to_log_file(log: &String, config: &Config) -> Result<(), String> {
    let mut _rel_path: String = String::new();
    let mut log_path = Path::new(&config.general.log_file);
  
    if config.general.log_file.len() > 0 {
        if log_path.is_relative() {
            let mut exe_path = match std::env::current_exe() {
                Ok(p)  => p, 
                Err(e) => return Err(format!("Error getting exe path, can not write to log.\nError: {}", e.to_string())),
            };

            exe_path.pop();
            let l_path = match exe_path.to_str() {
                Some(p) => p,
                None    => return Err(String::from("Path is not valid unicode, can not write to log.")),
            };

            _rel_path = format!("{}/{}", l_path, config.general.log_file);
            log_path = Path::new(&_rel_path);
        } 

        let mut open_opt: OpenOptions = OpenOptions::new();

        if log_path.exists() {
            open_opt.append(true);
        }
        else {
            open_opt.create(true).write(true);
        }


        let f = match open_opt.open(log_path) {
            Ok(f) => f,
            Err(e) => return Err(format!("Error opening log file: {}\nError: {}", log_path.to_str().unwrap(), e.to_string())),
        };

        let mut f = BufWriter::new(f);

        return match f.write_all(log.as_bytes()) {
            Ok(_)  => Ok(()),
            Err(e) => Err(format!("Error writing to log file: {}\nError: {}", log_path.to_str().unwrap(), e.to_string())),
        };
    }
    else {
        println!("{}", log);
    }

    Ok(())
}

/// If mail is enabled in config file
/// Mail will be sent according to
/// config file settings.
fn send_log_mail(log: &String, config: &Config, success: bool) -> String {
    if let Some(ref mail_conf) = config.mail_from_master {
        if mail_conf.enable {
            let mut msg: String = String::from("To: ");
            let mut to = mail_conf.to_addresses.clone();
            let mut auth: Mechanism = Mechanism::Plain;
            let mut mailer_build;

            let first_to = to.pop();
            if first_to.is_some() {
                msg = format!("{}{}", msg, first_to.unwrap());                
            }

            for m in to {
                msg = format!("{},{}", msg, m);
            }

            if success {
                msg = format!("{}\nSubject: {}: OK\n", msg, mail_conf.subject.clone());
            }
            else {
                msg = format!("{}\nSubject: {}: Error\n", msg, mail_conf.subject.clone());
            }

            msg = format!("{}{}", msg, log);

            to = mail_conf.to_addresses.clone();
            let mut mail_to: Vec<EmailAddress> = Vec::new();

            for m in to {
                mail_to.push(EmailAddress::new(m));
            }

            if mail_conf.authentication_mechanism == "login".to_string() {
                auth = Mechanism::Login;
            }
            else if mail_conf.authentication_mechanism == "crammd5".to_string() {
                auth = Mechanism::CramMd5;
            }



            // Use TLS certificates
            if mail_conf.tls_domain.is_some() {
                mailer_build = SmtpTransport::simple_builder(mail_conf.tls_domain.clone().unwrap()).unwrap();
            }
            else {
                mailer_build = SmtpTransport::builder_unencrypted_localhost().unwrap();
        
            }

            let email = SimpleSendableEmail::new(
                EmailAddress::new(mail_conf.from_sender_address.to_string()),
                mail_to,
                "message_id".to_string(),
                msg,
            );

            let mut mailer = mailer_build.credentials(Credentials::new(mail_conf.username.clone(), mail_conf.password.clone()))
                .smtp_utf8(true)
                .authentication_mechanism(auth).build();

            match mailer.send(&email) {
                Ok(_) => return "Master: Email sent OK".to_string(),
                Err(e) => return format!("Master: Error sending mail: {}", e.to_string()),
            }
        }
    }
    
    "Master: Email is not enabled".to_string()
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
            .conflicts_with_all(&["SNAPSHOT"])
            .takes_value(true)
            .help("Hostname for primary slave node.
Options VOULME and USER is reqiured
if not specified in config file."))
       .arg(Arg::with_name("INFO")
            .short("i")
            .long("info")
            .conflicts_with_all(&["VOLUME", "USER", "SLAVE", "SNAPSHOT"])
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
