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

use std::fs::File;
use std::io::prelude::*;

static CONF_FILE: &'static str = "ggsnap.conf";
static CONF_ETC_DIR: &'static str = "/etc/ggsnap.conf";
static CONF_ETC_SUB_DIR: &'static str = "/etc/ggsnap/ggsnap.conf";

/// Struct that holds all information from config file  
/// Config file ggsnap.conf is interpretated with toml format  
#[derive(Deserialize)]
pub struct Config {
    general: General,
    snapshot: Snapshot,
    mail_from_master: Option<MailFromMaster>,
}

/// Struct that holds information about sub section [general]  
/// in config file
#[derive(Deserialize)]
struct General {
    gluster_bin: String,
    ggsnap_slave_bin: String,
}

/// Struct that holds information about sub section [snapshot]  
/// in config file
#[derive(Deserialize)]
struct Snapshot {
    number_days_every_day: u32,
    number_months_with_two: u32,
    number_months_total: u32,
}

/// Struct that holds information about sub section [mail_from_master]  
/// in config file
#[derive(Deserialize)]
struct MailFromMaster {
    smtp_server: String,
    authentification_mechanism: String,
    username: String,
    password: String,
    from_sender_address: String,
    to_addresses: Vec<String>,
    enable: bool,
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
pub fn get_config() -> Result<Config, String> {
    let mut conf_content = String::new();

    if let Ok(mut f) = File::open(CONF_FILE) {
        match f.read_to_string(&mut conf_content) {
            Ok(_) => (),
            Err(e) => return Err(format!("Error: Can not read {} in current directory\n{}",
                                         CONF_FILE, e.to_string()))
        }
    }
    else if let Ok(mut f) = File::open(CONF_ETC_DIR) {
        match f.read_to_string(&mut conf_content) {
            Ok(_) => (),
            Err(e) => return Err(format!("Error: Can not read config file: {}\n{}",
                                         CONF_ETC_DIR, e.to_string()))
        }
    }
    else if let Ok(mut f) = File::open(CONF_ETC_SUB_DIR) {
        match f.read_to_string(&mut conf_content) {
            Ok(_) => (),
            Err(e) => return Err(format!("Error: Can not read config file: {}\n{}",
                                         CONF_ETC_SUB_DIR, e.to_string())),
        }
    }
    else {
        return Err(format!("Config file: {} is not found in current dir, /etc/ or /etc/ggsnap/", CONF_FILE))
    }

    parse_config(&conf_content)
}

/// Parses config string and returns a Config populated with
/// the content from string
fn parse_config(config_content: &String) -> Result<Config, String> {
    match toml::from_str(config_content.as_str()) {
        Ok(c) => Ok(c),
        Err(e) => Err(format!("Error parse config file: {}", e))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_file_is_missing() {
        assert_eq!(get_config(),
                   Err(format!("Config file: {} is not found in current dir, /etc/ or /etc/ggsnap/",
                               CONF_FILE)));
    }

    #[test]
    fn parse_config_file() {
        let conf = String::from("
            [general]
            gluster_bin = '/usr/sbin/gluster'
            ggsnap_slave_bin = '/root/ggsnap_slave'

            [snapshot]
            number_days_every_day = 10
            number_months_with_two = 3
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
        assert_eq!(c.snapshot.number_months_with_two, 3);
        assert_eq!(c.mail_from_master.unwrap().enable, true);

        let conf = String::from("
            [general]
            gluster_bin = '/usr/sbin/gluster'
            ggsnap_slave_bin = '/root/ggsnap_slave'

            [snapshot]
            number_days_every_day = 10
            number_months_with_two = 3
            number_months_total = 12
            ");

        let c = parse_config(&conf).unwrap();
        assert_eq!(c.general.gluster_bin, "/usr/sbin/gluster");
        assert_eq!(c.mail_from_master.is_none(), true);
    }
}
