/////////////////////////////////////////////////////////////////////////////////
//                                                                             //
//    ggsnap_utils, Common library for ggsnap and ggsnap_slave.                //
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

#[macro_use]
extern crate serde_derive;
extern crate toml;
extern crate chrono;

use std::fs::File;
use std::io::prelude::*;
use std::process::{ Command, Stdio };
use std::collections::HashSet;
use std::iter::FromIterator;
use chrono::prelude::*;

static CONF_FILE: &'static str = "ggsnap.conf";
static CONF_ETC_DIR: &'static str = "/etc/ggsnap.conf";
static CONF_ETC_SUB_DIR: &'static str = "/etc/ggsnap/ggsnap.conf";


/// Struct that holds all information from config file  
/// Config file ggsnap.conf is interpretated with toml format  
#[derive(Deserialize, Debug, PartialEq)]
pub struct Config {
    pub general: General,
    pub snapshot: Snapshot,
    pub mail_from_master: Option<MailFromMaster>,
}

impl Config {
    /// If config file is missing a default
    /// Config struct is returned
    pub fn default_config() -> Config {
        Config {
            general: General {
                gluster_bin: String::from("/usr/sbin/gluster"),
                ggsnap_slave_bin: String::from("/root/ggsnap_slave"),
                log_file: String::from("ggsnap.log"),
            },
            snapshot: Snapshot {
                number_days_every_day: 10,
                number_weeks_with_one: 10,
                number_months_total: 12,
                snapshot_name_prefix: Some(String::from("ggsnap")),
		delay_after_pause_before_snapshot: Some(0),
		delay_resume_geo_replication: Some(0),
                master_volume: None,
                slave_volume: None,
                slave_hostname: None,
                slave_user: None
            },
            mail_from_master: None
        }
    }
}

/// Struct that holds information about sub section [general]  
/// in config file
#[derive(Deserialize, Debug, PartialEq)]
pub struct General {
    pub gluster_bin: String,
    pub ggsnap_slave_bin: String,
    pub log_file: String,
}

/// Struct that holds information about sub section [snapshot]  
/// in config file
#[derive(Deserialize, Debug, PartialEq)]
pub struct Snapshot {
    pub number_days_every_day: u32,
    pub number_weeks_with_one: u32,
    pub number_months_total: u32,
    pub snapshot_name_prefix: Option<String>,
    pub delay_after_pause_before_snapshot: Option<u64>,
    pub delay_resume_geo_replication: Option<u64>,
    pub master_volume: Option<String>,
    pub slave_volume: Option<String>,
    pub slave_hostname: Option<String>,
    pub slave_user: Option<String>,
}

/// Struct that holds information about sub section [mail_from_master]  
/// in config file
#[derive(Deserialize, Debug, PartialEq)]
pub struct MailFromMaster {
    pub tls_domain: Option<String>,
    pub authentication_mechanism: String,
    pub username: String,
    pub password: String,
    pub from_sender_address: String,
    pub to_addresses: Vec<String>,
    pub subject: String, 
    pub enable: bool,
}


/// Type to describe type of read error
#[derive(PartialEq, Debug)]
pub enum ConfigReadErr {
    ConfigNotFound,
    ReadFileErr,
    ConfigParseErr,
    ConfigValueErr,
}

#[derive(PartialEq, Debug)]
pub enum HostType {
    Master,
    Slave,
}

/// Function checks for config file in three locations:  
/// * same directory as binary file  
/// * /etc/  
/// * /etc/ggsnap/  
///
/// Config file is parsed with the toml configuration file format
/// and a Result containing Config struct is returned containing all
/// configuration.
///
/// # Example  
/// ```
/// // To get _number_months_total from [snapshot]
/// // let conf = get_config().unwrap();
/// // println!("Total months: {}", config.snapshot.number_months_total);
/// ```
///  
/// If file is not found or an error occur while
/// trying to read config file, an error is returned
/// containing description of error.  
pub fn get_config() -> Result<Config, (ConfigReadErr, String)> {
    let mut conf_content = String::new();

    if let Ok(mut current_exe) = std::env::current_exe() {
        current_exe.pop();
        current_exe.push(CONF_FILE);
        if let Ok(mut f) = File::open(current_exe) {
            match f.read_to_string(&mut conf_content) {
                Ok(_) => (),
                Err(e) => return Err((ConfigReadErr::ReadFileErr,
                                      format!("Error: Can not read {} in current directory\n{}",
                                              CONF_FILE, e.to_string()))),
            }
        }
        else if let Ok(mut f) = File::open(CONF_ETC_DIR) {
            match f.read_to_string(&mut conf_content) {
                Ok(_) => (),
                Err(e) => return Err((ConfigReadErr::ReadFileErr,
                                      format!("Error: Can not read config file: {}\n{}",
                                              CONF_ETC_DIR, e.to_string()))),
            }
        }
        else if let Ok(mut f) = File::open(CONF_ETC_SUB_DIR) {
            match f.read_to_string(&mut conf_content) {
                Ok(_) => (),
                Err(e) => return Err((ConfigReadErr::ReadFileErr,
                                      format!("Error: Can not read config file: {}\n{}",
                                              CONF_ETC_SUB_DIR, e.to_string()))),
            }
        }
        else {
            return Err((ConfigReadErr::ConfigNotFound, format!("Config file: {} is not found in current dir, /etc/ or /etc/ggsnap/", CONF_FILE)))
        }
    }
    else {
        if let Ok(mut f) = File::open(CONF_ETC_DIR) {
            match f.read_to_string(&mut conf_content) {
                Ok(_) => (),
                Err(e) => return Err((ConfigReadErr::ReadFileErr,
                                      format!("Error: Can not read config file: {}\n{}",
                                              CONF_ETC_DIR, e.to_string()))),
            }
        }
        else if let Ok(mut f) = File::open(CONF_ETC_SUB_DIR) {
            match f.read_to_string(&mut conf_content) {
                Ok(_) => (),
                Err(e) => return Err((ConfigReadErr::ReadFileErr,
                                      format!("Error: Can not read config file: {}\n{}",
                                              CONF_ETC_SUB_DIR, e.to_string()))),
            }
        }
        else {
            return Err((ConfigReadErr::ConfigNotFound, format!("Config file: {} is not found in current dir, /etc/ or /etc/ggsnap/", CONF_FILE)))
        }
    }

    let config = match parse_config(&conf_content) {
        Ok(c) => c,
        Err(e) => return Err(e),
    };
    
    let month_30 = vec![4, 6, 9, 11];
    let today = Local::today();
    let mut date1 = today - chrono::Duration::days(config.snapshot.number_days_every_day as i64);
    date1 = date1 - chrono::Duration::weeks(config.snapshot.number_weeks_with_one as i64);
    let year = config.snapshot.number_months_total/12;
    let year_mod = config.snapshot.number_months_total%12;

    let mut date2 = today.offset().ymd(today.year() - year as i32, today.month(), today.day());

    if year_mod == today.month() {
        date2 = date2.offset().ymd(date2.year() - 1, 12, date2.day());
    }
    else if year_mod > today.month() {
        let month = 12 - (year_mod - date2.month());
        let mut day = date2.day();
        if day == 31 && month_30.contains(&month) {
            day = 30;
        }
        else if month == 2 && (date2.year() -1)%4 == 0 && day > 29 {
            day = 29;
        }
        else if month == 2 && day > 28 {
            day = 28;
        }
        date2 = date2.offset().ymd(date2.year() - 1, month, day);
    }
    else {
        let month = date2.month() - year_mod;
        let mut day = date2.day();
        if day == 31 && month_30.contains(&month) {
            day = 30;
        }
        else if month == 2 && (date2.year() -1)%4 == 0 && day > 29 {
            day = 29;
        }
        else if month == 2 && day > 28 {
            day = 28;
        }

        date2 = date2.offset().ymd(date2.year(), month, day);
    }

    let date1_s = format!("{}", date1.format("%Y%m%d"));
    let date2_s = format!("{}", date2.format("%Y%m%d"));

    if  date1_s >= date2_s {
        Ok(config)
    }
    else {
        Err((ConfigReadErr::ConfigValueErr, format!("    {}\n    {}\n    {}", 
                                                    "Error in parameters: number_days_every_day, number_weeks_with_one, number_months_total", 
                                                    "Value in number_months_total is too small or",  
                                                    "values in number_days_every_day and number_weeks_with_one are too large.")))
    }
    
}

/// Parses config string and returns a Config populated with
/// the content from string
fn parse_config(config_content: &String) -> Result<Config, (ConfigReadErr, String)> {
    match toml::from_str(config_content.as_str()) {
        Ok(c) => Ok(c),
        Err(e) => Err((ConfigReadErr::ConfigParseErr, format!("Error parse config file: {}", e)))
    }
}


/// Uses config file parameters in [snapshot]
/// to deside what to save and what to delete
/// On success a String containing removed snapshots
/// will be returned. On error, error message will be returned
pub fn remove_old_snapshots(config: &Config, host_type: HostType) -> Result<String, String> {
    let mut _snap_output: String = String::new();
    let mut _gluster_snaps: Vec<String> = Vec::new();
    let mut _rm_every_day: Vec<String> = Vec::new();
    let mut _rm_weeks_one: Vec<String> = Vec::new();
    let mut _rm_months_total: Vec<String> = Vec::new();
    let mut rm_tot: Vec<String> = Vec::new();
    let mut rm_tot_res: String = String::new();
    let mut del_err: bool = false;
    let cmd_out = Command::new(&config.general.gluster_bin)
                          .arg("snapshot")
                          .arg("list")
                          .output();

    match cmd_out {
        Ok(o) => {
            if o.status.success() {
                _snap_output = format!("{}", String::from_utf8_lossy(&o.stdout));
                _gluster_snaps = filter_gluster_snapshots(&_snap_output, &config, &host_type);

                _rm_every_day = get_remove_every_day(&config, &_gluster_snaps, &host_type);
                rm_tot.extend(_rm_every_day);
                _rm_weeks_one = get_remove_weeks_with_one(&config, &_gluster_snaps, &host_type);
                rm_tot.extend(_rm_weeks_one);
                _rm_months_total = get_remove_months_total(&config, &_gluster_snaps, &host_type);
                rm_tot.extend(_rm_months_total);
                rm_tot.sort();

                for l in rm_tot {
                    let yes_out = Command::new("/bin/yes").stdout(Stdio::piped()).spawn();

                    if yes_out.is_err() {
                        del_err = true;
                        if rm_tot_res.len() == 0 {
                            if host_type == HostType::Master {
                                rm_tot_res = format!("Master: Error executing command yes, to delete snapshots");
                            }
                            else {
                                rm_tot_res = format!("Slave: Error executing command yes, to delete snapshots");
                            }
                        }
                        else {
                            if host_type == HostType::Master {
                                rm_tot_res = format!("{}\nMaster: Error executing command yes, to delete snapshots", rm_tot_res);
                            }
                            else {
                                rm_tot_res = format!("{}\nSlave: Error executing command yes, to delete snapshots", rm_tot_res);
                            }
                        }

                        continue;
                    }

                    let yes_out = yes_out.unwrap();

                    let rm_out = Command::new(&config.general.gluster_bin)
                                         .arg("snapshot")
                                         .arg("delete")
                                         .arg(&l)
                                         .stdin(yes_out.stdout.unwrap())
                                         .output();

                    match rm_out {
                        Ok(o) => {
                            if o.status.success() {
                                if rm_tot_res.len() == 0 {
                                    if host_type == HostType::Master {
                                        rm_tot_res = format!("Master: {}\nMaster: {}", l, String::from_utf8_lossy(&o.stdout));
                                    }
                                    else {
                                        rm_tot_res = format!("Slave: {}\nSlave: {}", l, String::from_utf8_lossy(&o.stdout));
                                    }
                                }
                                else {
                                    if host_type == HostType::Master {
                                        rm_tot_res = format!("{}\nMaster: {}\nMaster: {}", rm_tot_res, l, String::from_utf8_lossy(&o.stdout));
                                    }
                                    else {
                                        rm_tot_res = format!("{}\nSlave: {}\nSlave: {}", rm_tot_res, l, String::from_utf8_lossy(&o.stdout));
                                    }
                                }
                            }
                            else {
                                del_err = true;
                                if rm_tot_res.len() == 0 {
                                    if host_type == HostType::Master {
                                        rm_tot_res = format!("Master: Error deleting snapshot: {}\nMaster: {}{}", l, 
                                                             String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
                                    }
                                    else {
                                        rm_tot_res = format!("Slave: Error deleting snapshot: {}\nSlave: {}{}", l, 
                                                             String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
                                    }
                                }
                                else {
                                    if host_type == HostType::Master {
                                        rm_tot_res = format!("{}\nMaster: Error deleting snapshot: {}\nMaster: {}{}", rm_tot_res, l, 
                                                             String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
                                    }
                                    else {
                                        rm_tot_res = format!("{}\nSlave: Error deleting snapshot: {}\nSlave: {}{}", rm_tot_res, l, 
                                                             String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            del_err = true;
                            if rm_tot_res.len() == 0 {
                                if host_type == HostType::Master {
                                    rm_tot_res = format!("Master: Error executing delete snapshot: {}\nMaster: {}", l, e.to_string());
                                }
                                else {
                                    rm_tot_res = format!("Slave: Error executing delete snapshot: {}\nSlave: {}", l, e.to_string());
                                }
                            }
                            else {
                                if host_type == HostType::Master {
                                    rm_tot_res = format!("{}\nMaster: Error executing delete snapshot: {}\nMaster: {}", rm_tot_res, l, e.to_string());
                                }
                                else {
                                    rm_tot_res = format!("{}\nSlave: Error executing delete snapshot: {}\nSlave: {}", rm_tot_res, l, e.to_string());
                                }
                            }
                        },
                    }
                }
            }
            else {
                return Err(format!("Error getting snapshots: {}{}", String::from_utf8_lossy(&o.stdout),
                            String::from_utf8_lossy(&o.stderr)))
            }
        },
        Err(e) => return Err(format!("Error executing command: gluster snapshot list\n{}", e.to_string())),
    }

    if del_err {
        Err(rm_tot_res)
    }
    else {
        Ok(rm_tot_res)
    }
}


/// Filters all snapshots done by ggsnap
/// and returns an ordered vector.
fn filter_gluster_snapshots(all_snaps: &String, config: &Config, host_type: &HostType) -> Vec<String> {
    let mut snap_prefix = config.snapshot.snapshot_name_prefix.clone().unwrap();
    let mut filtered_snaps: Vec<String> = Vec::new();

    if *host_type == HostType::Master {
        snap_prefix = format!("{}_{}_", snap_prefix, config.snapshot.master_volume.clone().unwrap());
    }
    else {
        snap_prefix = format!("{}_{}_", snap_prefix, config.snapshot.slave_volume.clone().unwrap());
    }

    let snap_pre_parts: Vec<&str> = snap_prefix.split("_").collect();
    let snap_pre_parts_len = snap_pre_parts.len();

    for l in all_snaps.split("\n") {
        if l.len() == (snap_prefix.len() + 15) && l.starts_with(&snap_prefix) {
            let snap_parts: Vec<&str> = l.split("_").collect();

            match snap_parts[snap_pre_parts_len-1].parse::<u32>() {
                Ok(_) => {
                    match snap_parts[snap_pre_parts_len].parse::<u32>() {
                        Ok(_) => filtered_snaps.push(l.to_string()),
                        Err(_) => (),
                    }
                },
                Err(_) => (),
            }
        }
    }

    filtered_snaps.sort();
    filtered_snaps
}

/// Returns all snapshots that should be deleted
/// accordning to config setting number_days_every_day
/// all_gluster_snaps should be the filtered list 
/// containing only snapshots that is made by ggsnap.
fn get_remove_every_day(config: &Config, all_gluster_snaps: &Vec<String>, host_type: &HostType) -> Vec<String> {
    let mut rm_snaps: HashSet<String> = HashSet::new();
    let mut dt = Local::now();
    let mut _snap_pre: String = String::new();

    for _i in 1..config.snapshot.number_days_every_day + 1 {
        if *host_type == HostType::Master {
            _snap_pre = format!("{}_{}_{}_", config.snapshot.snapshot_name_prefix.clone().unwrap(), 
                               config.snapshot.master_volume.clone().unwrap(), dt.format("%Y%m%d"));
        }        
        else {
            _snap_pre = format!("{}_{}_{}_", config.snapshot.snapshot_name_prefix.clone().unwrap(), 
                               config.snapshot.slave_volume.clone().unwrap(), dt.format("%Y%m%d"));
        }

        let found = all_gluster_snaps.iter().filter(|&& ref s| s.starts_with(&_snap_pre));
        let mut found_sort: Vec<&String> = found.collect();
        found_sort.sort_by(|a, b| b.cmp(a));
        let mut found_iter = found_sort.iter();
        found_iter.next();
        
        for &s in found_iter {
            rm_snaps.insert(s.clone());
        }

        dt = dt + chrono::Duration::days(-1);
    }

    Vec::from_iter(rm_snaps)
}

/// Returns all snapshots that should be deleted
/// accordning to config setting number_weeks_with_one
/// all_gluster_snaps should be the filtered list 
/// containing only snapshots that is made by ggsnap.
fn get_remove_weeks_with_one(config: &Config, all_gluster_snaps: &Vec<String>, host_type: &HostType) -> Vec<String> {
    let mut rm_snaps: HashSet<String> = HashSet::new();
    let mut date = Local::today();
    date = date + chrono::Duration::days(-((config.snapshot.number_days_every_day) as i64));
    let mut _date1 = Local::today();
    let mut _date2 = Local::today();

    if config.snapshot.number_weeks_with_one == 0 {
        return Vec::new();
    }

    for week_no in 0..config.snapshot.number_weeks_with_one {
        _date1 = date + chrono::Duration::weeks(-(week_no as i64));
        _date2 = _date1 + chrono::Duration::days(-6);

        let mut _snap_first: String = String::new();
        let mut _snap_last: String = String::new();
        
        if *host_type == HostType::Master {
            _snap_first = format!("{}_{}_{}_240000", config.snapshot.snapshot_name_prefix.clone().unwrap(), 
                                config.snapshot.master_volume.clone().unwrap(), _date1.format("%Y%m%d"));
        }
        else {
            _snap_first = format!("{}_{}_{}_240000", config.snapshot.snapshot_name_prefix.clone().unwrap(), 
                                config.snapshot.slave_volume.clone().unwrap(), _date1.format("%Y%m%d"));
        }
    
        if *host_type == HostType::Master {
            _snap_last = format!("{}_{}_{}_000000", config.snapshot.snapshot_name_prefix.clone().unwrap(), 
                                config.snapshot.master_volume.clone().unwrap(), _date2.format("%Y%m%d"));
        }
        else {
            _snap_last = format!("{}_{}_{}_000000", config.snapshot.snapshot_name_prefix.clone().unwrap(), 
                                config.snapshot.slave_volume.clone().unwrap(), _date2.format("%Y%m%d"));
        }
    
    
        let mut found: Vec<&String> = all_gluster_snaps.iter().filter(|&& ref s| *s < _snap_first && *s > _snap_last).collect();

        found.sort_by(|a, b| b.cmp(a));
        found.pop();

        for l in found {
            rm_snaps.insert(l.clone());
        }
    }

    Vec::from_iter(rm_snaps)
}


/// Returns all snapshots that should be deleted
/// accordning to config setting number_months_total
/// all_gluster_snaps should be the filtered list 
/// containing only snapshots that is made by ggsnap.
fn get_remove_months_total(config: &Config, all_gluster_snaps: &Vec<String>, host_type: &HostType) -> Vec<String> {
    let mut rm_snaps: HashSet<String> = HashSet::new();
    let mut last_month = Local::today();
    let mut month_start = Local::today();
    let mut _month_end = Local::today();
    
    month_start = month_start + chrono::Duration::days(-(config.snapshot.number_days_every_day as i64));
    month_start = month_start + chrono::Duration::weeks(-(config.snapshot.number_weeks_with_one as i64));
    _month_end = Local.ymd(month_start.year(), month_start.month(), 1);

    let mut year = last_month.year() - ((config.snapshot.number_months_total/12) as i32);
    let mut month = last_month.month() - (config.snapshot.number_months_total%12);
    let mut day = last_month.day();
    let months_30 = vec!(4, 6, 9, 11);
    let mut _snap_first: String = String::new();
    let mut _snap_last: String = String::new();

    if month < 1 {
        year = year -1;
        month = month + 12;
    }

    if month != 2 {
        if months_30.contains(&month) && day > 30 {
            day = 30;
        }
    }
    else {
        if year%4 == 0 && day > 29 {
            day = 29;
        }
        else if day > 28 {
            day = 28;
        }
    }

    last_month = Local.ymd(year, month, day);

    if *host_type == HostType::Master {
        _snap_last = format!("{}_{}_{}_000000", config.snapshot.snapshot_name_prefix.clone().unwrap(), 
                            config.snapshot.master_volume.clone().unwrap(), last_month.format("%Y%m%d"));
    }
    else {
        _snap_last = format!("{}_{}_{}_000000", config.snapshot.snapshot_name_prefix.clone().unwrap(), 
                            config.snapshot.slave_volume.clone().unwrap(), last_month.format("%Y%m%d"));
    }

    let old_snaps: Vec<&String> = all_gluster_snaps.iter().filter(|&& ref s| *s < _snap_last).collect();

    for o in old_snaps {
        rm_snaps.insert(o.clone().to_string());
    }

    loop {
        if month_start.year() < last_month.year() ||
           (month_start.year() == last_month.year() &&
            month_start.month() < last_month.month()) {
               break;
        }

        if *host_type == HostType::Master {
            _snap_first = format!("{}_{}_{}_240000", config.snapshot.snapshot_name_prefix.clone().unwrap(), 
                                 config.snapshot.master_volume.clone().unwrap(), month_start.format("%Y%m%d"));
        }
        else {
            _snap_first = format!("{}_{}_{}_240000", config.snapshot.snapshot_name_prefix.clone().unwrap(), 
                                 config.snapshot.slave_volume.clone().unwrap(), month_start.format("%Y%m%d"));
        }
    
        if *host_type == HostType::Master {
            _snap_last = format!("{}_{}_{}_000000", config.snapshot.snapshot_name_prefix.clone().unwrap(), 
                                config.snapshot.master_volume.clone().unwrap(), _month_end.format("%Y%m%d"));
        }
        else {
            _snap_last = format!("{}_{}_{}_000000", config.snapshot.snapshot_name_prefix.clone().unwrap(), 
                                config.snapshot.slave_volume.clone().unwrap(), _month_end.format("%Y%m%d"));
        }


        let mut all_in_month: Vec<&String> = all_gluster_snaps.iter().filter(|&& ref s| *s < _snap_first && *s > _snap_last).collect();

        all_in_month.sort_by(|a, b| b.cmp(a));
        all_in_month.pop();

        for s in all_in_month {
            rm_snaps.insert(s.clone().to_string());
        }

        let mut y = _month_end.year();
        let mut m = _month_end.month();
        let d = 1;

        if m == 1 {
            y -= 1;
            m = 12;
        }
        else {
            m -= 1;
        }

        _month_end = Local.ymd(y, m, d);

        if _month_end.year() == last_month.year() &&
           _month_end.month() == last_month.month() {
            _month_end = last_month;
        }

        let mut d = 31;

        if months_30.contains(&_month_end.month()) {
            d = 30;
        }
        else if _month_end.month() == 2 && _month_end.year()%4 == 0 {
            d = 29;
        }
        else if _month_end.month() == 2 {
            d = 28;
        }

        month_start = Local.ymd(_month_end.year(), _month_end.month(), d);

    }

    Vec::from_iter(rm_snaps)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_file_is_missing() {
        assert_eq!(get_config(),
                   Err((ConfigReadErr::ConfigNotFound, format!("Config file: {} is not found in current dir, /etc/ or /etc/ggsnap/",
                               CONF_FILE))));
    }

    #[test]
    fn parse_config_file() {
        let conf = String::from("
            [general]
            gluster_bin = '/usr/sbin/gluster'
            ggsnap_slave_bin = '/root/ggsnap_slave'

            [snapshot]
            number_days_every_day = 10
            number_weeks_with_one = 10
            number_months_total = 12

            [mail_from_master]
            smtp_server = 'mysmtp.server.com'
            authentification_mechanism = 'plain'
            username = 'foobar'
            password = 'noob'
            from_sender_address = 'aa@bb.cc'
            to_addresses = [ 'foobar@foobar.com', 'noob@noob.com' ]
            enable = true
            ");

        let c = parse_config(&conf).unwrap();
        assert_eq!(c.general.ggsnap_slave_bin, "/root/ggsnap_slave");
        assert_eq!(c.snapshot.number_weeks_with_one, 10);
        assert_eq!(c.mail_from_master.unwrap().enable, true);

        let conf = String::from("
            [general]
            gluster_bin = '/usr/sbin/gluster'
            ggsnap_slave_bin = '/root/ggsnap_slave'

            [snapshot]
            number_days_every_day = 10
            number_weeks_with_one = 10
            number_months_total = 12
            ");

        let c = parse_config(&conf).unwrap();
        assert_eq!(c.general.gluster_bin, "/usr/sbin/gluster");
        assert_eq!(c.mail_from_master.is_none(), true);
    }

    #[test]
    fn get_every_day() {
        let mut dates: Vec<String> = Vec::new();
        let d  = Local::today();
        dates.push(format!("{}", d.format("%Y%m%d")));

        for i in 1..12 {
            dates.push(format!("{}", (d + chrono::Duration::days(-i)).format("%Y%m%d")));
        }

        let mut config = Config::default_config();
        config.snapshot.slave_volume = Some(String::from("v_o_l"));
        let s = format!(
"ggsnap_v_o_l_{}_151810
ggsnap_v_o_l_{}_151811
ggsnap_v_o_l_{}_101811
ggsnap_v_o_l_{}_091811
ggsnap_v_o_l_{}_131011
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_111011
ggsnap_v_o_l_{}_001011
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_230000
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_181011
ggsnap_v_o_l_{}_141011
ggsnap_v_o_l_{}_141011
ggsnap_v_o_l_{}_101011
ggsnap_v_o_l_{}_191011
ggsnap_v_o_l_{}_221011
ggsnap_v_o_l_{}_231011
ggsnap_v_o_l_{}_231011
ggsnap_v_o_l_{}_011011
ggsnap_v_o_l_{}_041011
ggsnap_v_o_l_{}_051011
ggsnap_v_o_l_{}_061011
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_085228
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_121011
ggsnap_v_o_l_{}_115228", 
            dates[1], dates[1], dates[1], dates[1], dates[2], dates[2], dates[3], dates[3], dates[3], dates[3],
            dates[4], dates[5], dates[6], dates[7], dates[7], dates[8], dates[8], dates[8], dates[8], dates[8],
            dates[9], dates[9], dates[9], dates[9], dates[9], dates[10], dates[10], dates[11], dates[11], dates[11]);
        let mut snaps: Vec<String> = Vec::new();
        for l in s.split("\n") {
            snaps.push(l.to_string());
        }
        snaps.sort();
        let mut res: Vec<String> = Vec::new();
        let r: String = format!(
"ggsnap_v_o_l_{}_151810
ggsnap_v_o_l_{}_101811
ggsnap_v_o_l_{}_091811
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_111011
ggsnap_v_o_l_{}_001011
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_141011
ggsnap_v_o_l_{}_141011
ggsnap_v_o_l_{}_101011
ggsnap_v_o_l_{}_191011
ggsnap_v_o_l_{}_221011
ggsnap_v_o_l_{}_011011
ggsnap_v_o_l_{}_041011
ggsnap_v_o_l_{}_051011
ggsnap_v_o_l_{}_061011",
            dates[1], dates[1], dates[1], dates[2], dates[3], dates[3], dates[3], dates[7], dates[8], 
            dates[8], dates[8], dates[8], dates[9], dates[9], dates[9], dates[9]); 
        for l in r.split("\n") {
            res.push(l.to_string());
        }

        let mut days = get_remove_every_day(&config, &snaps, &HostType::Slave);
        days.sort();
        res.sort();
        assert_eq!(days, res);
    }


    #[test]
    fn get_weeks_with_one() {
        let mut dates: Vec<String> = Vec::new();
        let d  = Local::today();
        dates.push(format!("{}", d.format("%Y%m%d")));

        for i in 1..12 {
            dates.push(format!("{}", (d + chrono::Duration::days(-i)).format("%Y%m%d")));
        }

        let mut week1_1: String = String::from("");
        let mut week1_2: String = String::from("");
        let mut week2_1: String = String::from("");
        let mut week3_1: String = String::from("");        
        let mut week3_2: String = String::from("");
        let mut week3_3: String = String::from("");
        let mut week4_1: String = String::from("");
        let mut week5_1: String = String::from("");
        let mut week6_1: String = String::from("");
        let mut week7_1: String = String::from("");
        let mut week8_1: String = String::from("");
        let mut week8_2: String = String::from("");
        let mut week8_3: String = String::from("");
        let mut week8_4: String = String::from("");
        let mut week9_1: String = String::from("");
        let mut week10_1: String = String::from("");
        let mut week11_1: String = String::from("");
        let mut week11_2: String = String::from("");
        
        let d2 = d + chrono::Duration::days(-11);
        let mut d3 = d2.clone();

        week1_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d3 + chrono::Duration::days(-2);
        week1_2 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-1);
        d3 = d3 + chrono::Duration::days(-4);
        week2_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-2);
        d3 = d3 + chrono::Duration::days(-1);
        week3_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));        
        d3 = d3 + chrono::Duration::days(-4);
        week3_2 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d3 + chrono::Duration::days(-5);
        week3_3 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-3);
        d3 = d3 + chrono::Duration::days(-2);
        week4_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-4);
        d3 = d3 + chrono::Duration::days(-6);
        week5_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-5);
        d3 = d3 + chrono::Duration::days(-1);
        week6_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-6);
        d3 = d3 + chrono::Duration::days(-1);
        week7_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-7);
        d3 = d3 + chrono::Duration::days(-1);
        week8_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d3 + chrono::Duration::days(-2);
        week8_2 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d3 + chrono::Duration::days(-3);
        week8_3 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d3 + chrono::Duration::days(-5);
        week8_4 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-8);
        d3 = d3 + chrono::Duration::days(-4);
        week9_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-9);
        d3 = d3 + chrono::Duration::days(-2);
        week10_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-10);
        d3 = d3 + chrono::Duration::days(-3);
        week11_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d3 + chrono::Duration::days(-5);
        week11_2 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));

        let mut config = Config::default_config();
        config.snapshot.slave_volume = Some(String::from("v_o_l"));
        let s = format!(
"ggsnap_v_o_l_{}_151810
ggsnap_v_o_l_{}_151811
ggsnap_v_o_l_{}_101811
ggsnap_v_o_l_{}_091811
ggsnap_v_o_l_{}_131011
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_111011
ggsnap_v_o_l_{}_001011
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_230000
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_181011
ggsnap_v_o_l_{}_141011
ggsnap_v_o_l_{}_141011
ggsnap_v_o_l_{}_101011
ggsnap_v_o_l_{}_191011
ggsnap_v_o_l_{}_221011
ggsnap_v_o_l_{}_231011
ggsnap_v_o_l_{}_231011
ggsnap_v_o_l_{}_011011
ggsnap_v_o_l_{}_041011
ggsnap_v_o_l_{}_051011
ggsnap_v_o_l_{}_061011
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_085228
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_121011
ggsnap_v_o_l_{}_115228
ggsnap_v_o_l_{}_165228
ggsnap_v_o_l_{}_031034
ggsnap_v_o_l_{}_122311
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_051011
ggsnap_v_o_l_{}_221011
ggsnap_v_o_l_{}_191011
ggsnap_v_o_l_{}_165228
ggsnap_v_o_l_{}_141034
ggsnap_v_o_l_{}_124311
ggsnap_v_o_l_{}_041011
ggsnap_v_o_l_{}_051011
ggsnap_v_o_l_{}_061011
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_085228
ggsnap_v_o_l_{}_081034
ggsnap_v_o_l_{}_121011
ggsnap_v_o_l_{}_115228", 
            dates[1], dates[1], dates[1], dates[1], dates[2], dates[2], dates[3], dates[3], dates[3], dates[3],
            dates[4], dates[5], dates[6], dates[7], dates[7], dates[8], dates[8], dates[8], dates[8], dates[8],
            dates[9], dates[9], dates[9], dates[9], dates[9], dates[10], dates[10], dates[11], dates[11], dates[11],
            week1_1, week1_2, week2_1, week3_1, week3_2, week3_3, week4_1, week5_1, week6_1, week7_1, week8_1, 
            week8_2, week8_3, week8_4, week9_1, week10_1, week11_1, week11_2);


        let mut snaps: Vec<String> = Vec::new();
        for l in s.split("\n") {
            snaps.push(l.to_string());
        }
        snaps.sort();
        
        let mut res: Vec<String> = Vec::new();
        let mut r: String = String::new();
        r = format!(
"ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_085228
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_121011
ggsnap_v_o_l_{}_115228
ggsnap_v_o_l_{}_165228
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_191011
ggsnap_v_o_l_{}_165228
ggsnap_v_o_l_{}_041011
ggsnap_v_o_l_{}_061011
ggsnap_v_o_l_{}_085228", 
            dates[10], dates[10], dates[11], dates[11], dates[11],
            week1_1, week3_1, week4_1, week5_1, week8_1, week8_3, week8_4);

        for l in r.split("\n") {
            res.push(l.to_string());
        }


        let mut days = get_remove_weeks_with_one(&config, &snaps, &HostType::Slave);
        days.sort();
        res.sort();

        assert_eq!(days, res);
    }


    #[test]
    fn get_total_length() {
        let mut dates: Vec<String> = Vec::new();
        let d  = Local::today();
        dates.push(format!("{}", d.format("%Y%m%d")));

        for i in 1..12 {
            dates.push(format!("{}", (d + chrono::Duration::days(-i)).format("%Y%m%d")));
        }

        let mut week1_1: String = String::from("");
        let mut week1_2: String = String::from("");
        let mut week2_1: String = String::from("");
        let mut week3_1: String = String::from("");        
        let mut week3_2: String = String::from("");
        let mut week3_3: String = String::from("");
        let mut week4_1: String = String::from("");
        let mut week5_1: String = String::from("");
        let mut week6_1: String = String::from("");
        let mut week7_1: String = String::from("");
        let mut week8_1: String = String::from("");
        let mut week8_2: String = String::from("");
        let mut week8_3: String = String::from("");
        let mut week8_4: String = String::from("");
        let mut week9_1: String = String::from("");
        let mut week10_1: String = String::from("");
        let mut week11_1: String = String::from("");
        let mut week11_2: String = String::from("");
        let mut month2_1: String = String::from("");
        let mut month2_2: String = String::from("");
        let mut month2_3: String = String::from("");        
        let mut month4_1: String = String::from("");
        let mut month5_1: String = String::from("");
        let mut month6_1: String = String::from("");
        let mut month7_1: String = String::from("");
        let mut month7_2: String = String::from("");
        let mut month7_3: String = String::from("");
        let mut month7_4: String = String::from("");
        let mut month8_1: String = String::from("");
        let mut month9_1: String = String::from("");
        let mut month10_1: String = String::from("");
        let mut month11_1: String = String::from("");
        let mut month12_1: String = String::from("");
        let mut month13_1: String = String::from("");
        let mut month14_1: String = String::from("");
        let mut month14_2: String = String::from("");
        let mut month14_3: String = String::from("");

        let d2 = d + chrono::Duration::days(-11);
        let mut d3 = d2.clone();

        week1_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d3 + chrono::Duration::days(-2);
        week1_2 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-1);
        d3 = d3 + chrono::Duration::days(-4);
        week2_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-2);
        d3 = d3 + chrono::Duration::days(-1);
        week3_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));        
        d3 = d3 + chrono::Duration::days(-4);
        week3_2 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d3 + chrono::Duration::days(-5);
        week3_3 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-3);
        d3 = d3 + chrono::Duration::days(-2);
        week4_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-4);
        d3 = d3 + chrono::Duration::days(-6);
        week5_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-5);
        d3 = d3 + chrono::Duration::days(-1);
        week6_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-6);
        d3 = d3 + chrono::Duration::days(-1);
        week7_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-7);
        d3 = d3 + chrono::Duration::days(-1);
        week8_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d3 + chrono::Duration::days(-2);
        week8_2 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d3 + chrono::Duration::days(-3);
        week8_3 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d3 + chrono::Duration::days(-5);
        week8_4 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-8);
        d3 = d3 + chrono::Duration::days(-4);
        week9_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-9);
        d3 = d3 + chrono::Duration::days(-2);
        week10_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d2 + chrono::Duration::weeks(-10);
        d3 = d3 + chrono::Duration::days(-3);
        week11_1 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        d3 = d3 + chrono::Duration::days(-5);
        week11_2 = format!("{}", d3.offset().ymd(d3.year(), d3.month(), d3.day()).format("%Y%m%d"));
        let mut y = d3.year();
        let mut m = d3.month();
        let mut d = 14;

        m -= 1;
        if m > 1 {
            month2_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        else {
            y -= 1;
            m = 12;
            month2_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        month2_2 = Local.ymd(y, m, 8).format("%Y%m%d").to_string();
        month2_3 = Local.ymd(y, m, 3).format("%Y%m%d").to_string();
        month2_3 = Local.ymd(y, m, 3).format("%Y%m%d").to_string();

        m -= 2;
        if m > 2 {
            month4_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        else if m == 1{
            y -= 1;
            m = 11;
            month4_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        else {
            y -= 1;
            m = 12;
            month4_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }

        m -= 1;
        if m > 1 {
            month5_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        else {
            y -= 1;
            m = 12;
            month5_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }


        m -= 1;
        if m > 1 {
            month6_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        else {
            y -= 1;
            m = 12;
            month6_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }

        m -= 1;
        if m > 1 {
            month7_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        else {
            y -= 1;
            m = 12;
            month7_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        month7_2 = Local.ymd(y, m, 12).format("%Y%m%d").to_string();
        month7_3 = Local.ymd(y, m, 9).format("%Y%m%d").to_string();
        month7_4 = Local.ymd(y, m, 1).format("%Y%m%d").to_string();

        m -= 1;
        if m > 1 {
            month8_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        else {
            y -= 1;
            m = 12;
            month8_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }

        m -= 1;
        if m > 1 {
            month9_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        else {
            y -= 1;
            m = 12;
            month9_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }

        m -= 1;
        if m > 1 {
            month10_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        else {
            y -= 1;
            m = 12;
            month10_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }

        m -= 1;
        if m > 1 {
            month11_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        else {
            y -= 1;
            m = 12;
            month11_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }

        m -= 1;
        if m > 1 {
            month12_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        else {
            y -= 1;
            m = 12;
            month12_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }

        m -= 1;
        if m > 1 {
            month13_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        else {
            y -= 1;
            m = 12;
            month13_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }

        m -= 1;
        if m > 1 {
            month14_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }
        else {
            y -= 1;
            m = 12;
            month14_1 = Local.ymd(y, m, d).format("%Y%m%d").to_string();
        }

        month14_2 = Local.ymd(y, m, 11).format("%Y%m%d").to_string();
        month14_2 = Local.ymd(y, m, 8).format("%Y%m%d").to_string();
        month14_3 = Local.ymd(y, m, 2).format("%Y%m%d").to_string();

        let mut config = Config::default_config();
        config.snapshot.slave_volume = Some(String::from("v_o_l"));
        let s = format!(
"ggsnap_v_o_l_{}_151810
ggsnap_v_o_l_{}_151811
ggsnap_v_o_l_{}_101811
ggsnap_v_o_l_{}_091811
ggsnap_v_o_l_{}_131011
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_111011
ggsnap_v_o_l_{}_001011
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_230000
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_181011
ggsnap_v_o_l_{}_141011
ggsnap_v_o_l_{}_141011
ggsnap_v_o_l_{}_101011
ggsnap_v_o_l_{}_191011
ggsnap_v_o_l_{}_221011
ggsnap_v_o_l_{}_231011
ggsnap_v_o_l_{}_231011
ggsnap_v_o_l_{}_011011
ggsnap_v_o_l_{}_041011
ggsnap_v_o_l_{}_051011
ggsnap_v_o_l_{}_061011
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_085228
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_121011
ggsnap_v_o_l_{}_115228
ggsnap_v_o_l_{}_165228
ggsnap_v_o_l_{}_031034
ggsnap_v_o_l_{}_122311
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_051011
ggsnap_v_o_l_{}_221011
ggsnap_v_o_l_{}_191011
ggsnap_v_o_l_{}_165228
ggsnap_v_o_l_{}_141034
ggsnap_v_o_l_{}_124311
ggsnap_v_o_l_{}_041011
ggsnap_v_o_l_{}_051011
ggsnap_v_o_l_{}_061011
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_085228
ggsnap_v_o_l_{}_081034
ggsnap_v_o_l_{}_121011
ggsnap_v_o_l_{}_115228
ggsnap_v_o_l_{}_165228
ggsnap_v_o_l_{}_031034
ggsnap_v_o_l_{}_122311
ggsnap_v_o_l_{}_091011
ggsnap_v_o_l_{}_051011
ggsnap_v_o_l_{}_221011
ggsnap_v_o_l_{}_191011
ggsnap_v_o_l_{}_165228
ggsnap_v_o_l_{}_141034
ggsnap_v_o_l_{}_124311
ggsnap_v_o_l_{}_041011
ggsnap_v_o_l_{}_051011
ggsnap_v_o_l_{}_061011
ggsnap_v_o_l_{}_081011
ggsnap_v_o_l_{}_085228
ggsnap_v_o_l_{}_081034
ggsnap_v_o_l_{}_121011
ggsnap_v_o_l_{}_115228
ggsnap_v_o_l_{}_115228", 
            dates[1], dates[1], dates[1], dates[1], dates[2], dates[2], dates[3], dates[3], dates[3], dates[3],
            dates[4], dates[5], dates[6], dates[7], dates[7], dates[8], dates[8], dates[8], dates[8], dates[8],
            dates[9], dates[9], dates[9], dates[9], dates[9], dates[10], dates[10], dates[11], dates[11], dates[11],
            week1_1, week1_2, week2_1, week3_1, week3_2, week3_3, week4_1, week5_1, week6_1, week7_1, week8_1, 
            week8_2, week8_3, week8_4, week9_1, week10_1, week11_1, week11_2, month2_1, month2_2, month2_3, month4_1,
            month5_1, month6_1, month7_1, month7_2, month7_3, month7_4, month8_1, month9_1, month10_1, month11_1, 
            month12_1, month13_1, month14_1, month14_2, month14_3);
        //test
//        assert_eq!(s, String::new());

        let mut snaps: Vec<String> = Vec::new();
        for l in s.split("\n") {
            snaps.push(l.to_string());
        }
        snaps.sort();
        
        let mut res: Vec<String> = Vec::new();
        let mut r: String = String::new();
        r = format!(
"ggsnap_v_o_l_{}_121011
ggsnap_v_o_l_{}_031034
ggsnap_v_o_l_{}_165228
ggsnap_v_o_l_{}_165228
ggsnap_v_o_l_{}_141034
ggsnap_v_o_l_{}_191011
ggsnap_v_o_l_{}_081034
ggsnap_v_o_l_{}_121011
ggsnap_v_o_l_{}_115228
ggsnap_v_o_l_{}_115228
ggsnap_v_o_l_{}_085228
ggsnap_v_o_l_{}_081011", 
            week11_1, month2_2, month2_1, month7_2, month7_3, month7_1, month13_1, month14_1, month14_2, month14_3, month12_1, month11_1);

        for l in r.split("\n") {
            res.push(l.to_string());
        }


        let mut days = get_remove_months_total(&config, &snaps, &HostType::Slave);
        days.sort();
        res.sort();

        assert_eq!(days, res);
    }
}
