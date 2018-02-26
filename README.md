# gluster-geo-snapshot

This is a software that creates and saves snapshots on a gluster geo-replicated cluster.  
Snapshots are created and saved according to specified scheme in ggsnap.conf file.  
Snapshots are created on both master and slave cluster.  

## Usage
There are two options to create snapshots:  
 * Use valid options on command line starting ggsnap
 ```
 ggsnap --volume master-volume --slave slave-volume --host slave-host-name --user slave-host-user
 ```
 If some option is left out value will be read from config file.
 If value is missing in config file, you will get an error.
 * Use information from config file and start ggsnap with:
 ```
 ggsnap --create-snapshots
 ```
 If all required information is not in config file, you will get an error.

## Compilation
gluster-geo-snapshot is written in rust: <https://www.rust-lang.org>  
To compile install both rust and cargo packages, on centos 7:  

```
sudo yum install rust
sudo yum install cargo
```

gluster-geo-snapshot contains three rust projects:  
* ggsnap - program that should be run on master node
* ggsnap_slave - program that should run on slave node
* ggsnap_utils - common library for both ggsnap and ggsnap_slave

Compilation; ggsnap and ggsnap_slave needs to be compiled:  
```
cd ggsnap
cargo build --release
# binary will be found in ./target/release/ggsnap

cd ggsnap_slave
cargo build --release
# binary will be found in ./target/release/ggsnap_slave
```

## ggsnap.conf file
The config file for gluster-geo-snapshot is called: ggsnap.conf  
Toml configuration file format is used in config file.  
Config file should be used for both ggsnap and ggsnap_slave.  
Put config file in same directory as binary ggsnap and ggsnap_slave.  
If file is not found there it will first look in: /etc/ggsnap.conf  
and secondly in /etc/ggsnap/ggsnap.conf  
If config file is not found, default settings will be used.  
Config file showing the default settings:  
```
[general]
# All values are required
# Path to binary gluster change if installed somewere else
gluster_bin = "/usr/sbin/gluster"

# Path to ggsnap_slave on slave node change if path is different
ggsnap_slave_bin = "/root/ggsnap_slave"


# Settings for how snapshots should be saved
[snapshot]
# All values are required except the last one marked as optional
# Number of days that snapshot should be saved every day from today
number_days_every_day = 10

# Number of months that two snapshot per month should be saved after days.
number_months_with_two = 3

# Number of months in total; the rest of the months one snapshot is saved
number_months_total = 12

# All the following values are optional,
# one or more values can be specified
# If options for these values are not specified on command line
# the values will be used from this file
master_volume = ""
slave_volume = ""
slave_hostname = ""
slave_user = ""


# Mail settings for sending status mails every time a snapshot is done.
# Master node is sending mail, slave node do not use this setting
# Mail is disabled by default
# All values are optional but if specified all values
# must be specified
[mail_from_master]
# Smtp server to use when sending mails
smtp_server = ""
authentification_mechanism = "plain"
# Valid values are: plan, login, crammd5
username = ""
password = ""
# Mail address that mail will be sent from
from_sender_address = ""
# List of mail addresses to send to
to_addresses = [ "foobar@foobar.com", "noob@noob.com" ]
# Enable or diable sending mail (default disabled)
enable = false # true

```

If ggsnap.conf is missing in checked directories, default values will be used.  
If config file is wrongly formated, you will get an error and snapshots will  
not be made until config file is corrected.  

## Setup
ggsnap should be in main master node and ggsnap_slave in main slave node.  
Setup a cron job to run ggsnap on master.  
Root crontab:  
```
0 22 * * * /root/ggsnap
```

ggsnap calls ggsnap_slave over ssh, so setup password less login for  
root from master node to slave node:  
```
# ssh-keygen
# ssh-copy-id slave-node-name
```

Use command in authorized_keys to limit what master root can do on slave  
```
# In slave node in /root/.ssh/authorized_keys
from="ip address master node",command="/root/ggsnap_slave" ssh-rsa FF09AD04322....09976DD root@master-node
```
