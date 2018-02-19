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


extern crate regex;

use std::process::Command;
use self::regex::Regex;

pub struct SnapStat {
    snapnames: Vec<String>,
}

impl SnapStat {
    pub fn new(gluster_snap_list: String) -> SnapStat {
        let mut snap_list: Vec<String> = Vec::new();
        let re_snap = Regex::new(r"^snap_.*_\d{8}_\d{6}$").unwrap();

        for line in gluster_snap_list.split("\n") {
            if re_snap.is_match(line) {
                snap_list.push(line.to_string());
            }
        }

        SnapStat { snapnames: snap_list }
    }

    pub fn len(&self) -> usize {
        self.snapnames.len()
    }

    pub fn newest_snap(&self) -> String {
        let mut newest_snap_date = 0;
        let mut newest_snap_time = 0;
        let mut newest_snap: String = String::from("");

        for snap in self.snapnames.iter() {
            let snap_split: Vec<&str> = snap.split('_').collect();
            let snap_d: i32 = snap_split[2].parse::<i32>().unwrap();
            let snap_t: i32 = snap_split[3].parse::<i32>().unwrap();
            
            if snap_d > newest_snap_date {
                newest_snap_date = snap_d;
                newest_snap_time = snap_t;
                newest_snap = snap.clone();
            }
            else if snap_d == newest_snap_date && snap_t > newest_snap_time {
                newest_snap_time = snap_t;
                newest_snap = snap.clone();
            }
        }

        newest_snap
    }

    pub fn oldest_snap(&self) -> String {
        let mut oldest_snap_date = 99999999;
        let mut oldest_snap_time = 999999;
        let mut oldest_snap: String = String::from("");

        for snap in self.snapnames.iter() {
            let snap_split: Vec<&str> = snap.split('_').collect();
            let snap_d: i32 = snap_split[2].parse::<i32>().unwrap();
            let snap_t: i32 = snap_split[3].parse::<i32>().unwrap();
            
            if snap_d < oldest_snap_date {
                oldest_snap_date = snap_d;
                oldest_snap_time = snap_t;
                oldest_snap = snap.clone();
            }
            else if snap_d == oldest_snap_date && snap_t < oldest_snap_time {
                oldest_snap_time = snap_t;
                oldest_snap = snap.clone();
            }
        }

        oldest_snap
    }

    pub fn number_diff(&self, other: &SnapStat) -> u32 {
        let mut no_diff: u32 = 0;

        for snap in self.snapnames.iter() {
            if !other.snapnames.contains(&snap) {
                no_diff += 1;
            }
        }

        for snap in other.snapnames.iter() {
            if !self.snapnames.contains(&snap) {
                no_diff += 1;
            }
        }

        no_diff
    }

}

pub fn get_statistics() -> SnapStat {
    let output = Command::new("/usr/sbin/gluster")
                         .arg("snapshot")
                         .arg("list")
                         .output()
                         .expect("Error executing command: gluster snapshot list");

    let stdout: String = String::from_utf8_lossy(&output.stdout).to_string();
    SnapStat::new(stdout)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapstat_creation() {
        let gluster_snap: String = String::from(
"snap_vol_20180212_155212_GMT-2018.02.12-14.52.47
snap_20180213_092734_master
snap_vol_20180214_095616
snap_vol_20180214_101635
snap_vol_20180216_114403
snap_vol_20180216_115150
snap_vol_20180216_115548
snap_vol_20180216_115928
snap_vol_20180216_120438");

        let gluster_snap2: String = String::from(
"snap_vol_20180214_095616
snap_vol_20180214_101635
snap_vol_20180216_114403
snap_vol_20180216_115150
snap_vol_20180216_115548
snap_vol_20180216_115928
snap_vol_20180216_120438");

        let gluster_snap3: String = String::from(
"snap_vol_20180214_095616
snap_vol_20180214_101635
snap_vol_20180216_115150
snap_vol_20180216_115548
snap_vol_20180216_115928
snap_vol_20180216_120438");

        let gluster_snap4: String = String::from(
"snap_vol_20180214_095616
snap_vol_20180214_101635
snap_vol_20180216_114403
snap_vol_20180216_115150
snap_vol_20180216_115548
snap_vol_20180216_115924
snap_vol_20180216_120438");


        let stat = SnapStat::new(gluster_snap);
        let stat2 = SnapStat::new(gluster_snap2);
        let stat3 = SnapStat::new(gluster_snap3);
        let stat4 = SnapStat::new(gluster_snap4);

        assert_eq!(stat.len(), 7);
        assert_eq!(stat.newest_snap(), "snap_vol_20180216_120438");
        assert_eq!(stat.oldest_snap(), "snap_vol_20180214_095616");
        assert_eq!(stat.number_diff(&stat2), 0);
        assert_eq!(stat.number_diff(&stat3), 1);
        assert_eq!(stat.number_diff(&stat4), 2);
    }
}
