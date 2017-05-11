use sha1;
use ioreg;
use clap::ArgMatches;
use std::path::PathBuf;
use std::fs;
use std::io::Read;
use Result;

#[derive(Debug)]
pub struct UsbDevice {
    pub vendor_id: u16,
    pub product_id: u16,
    pub vendor_string: String,
    pub product_string: String,
    pub serial_number: String,    
    pub location_id: Option<i64>,
}

impl UsbDevice {
    pub fn hash(&self) -> String {
        let mut h = sha1::Sha1::new();
        h.update(self.vendor_string.as_bytes());
        h.update(self.product_string.as_bytes());
        h.update(self.serial_number.as_bytes());
        h.digest().to_string()
    }
}

pub trait Device {
    fn usb(&self) -> &UsbDevice;
    fn hash(&self) -> String {
        self.usb().hash()
    }
    fn is_unknown(&self) -> bool { false }
    fn device_type(&self) -> Option<&str>;
    fn serial_path(&self) -> Option<String> { None }
    fn msd_path(&self) -> Option<PathBuf> { None }
    fn openocd_serial(&self) -> Option<String> { None }
}

pub struct UnknownDevice {
    usb: UsbDevice,
}

impl Device for UnknownDevice {
    fn usb(&self) -> &UsbDevice {
        &self.usb
    }
    fn is_unknown(&self) -> bool { 
       true
    }
    fn device_type(&self) -> Option<&str> {
        None
    }
}

pub struct JLinkDevice {
    usb: UsbDevice,
}

impl Device for JLinkDevice {
    fn usb(&self) -> &UsbDevice {
        &self.usb
    }

    fn device_type(&self) -> Option<&str> {
        Some("JLink")
    }

    fn serial_path(&self) -> Option<String> {
        Some(format!("/dev/cu.usbmodem{}{}", 
            format!("{:x}", self.usb.location_id.unwrap_or(0)).replace("0",""),
            1,
        ))
    }

    fn openocd_serial(&self) -> Option<String> {
        Some(format!("jlink_serial {}", self.usb.serial_number))
    }
}

pub struct StLinkV2Device {
    usb: UsbDevice,
}

impl Device for StLinkV2Device {
    fn usb(&self) -> &UsbDevice {
        &self.usb
    }

    fn device_type(&self) -> Option<&str> {
        Some("STLinkV2")
    }

    fn openocd_serial(&self) -> Option<String> {
        Some(format!("hla_serial {}", self.usb.serial_number))
    }    
}

pub struct StLinkV21Device {
    usb: UsbDevice,
}

impl Device for StLinkV21Device {
    fn usb(&self) -> &UsbDevice {
        &self.usb
    }

    fn device_type(&self) -> Option<&str> {
        Some("STLinkV21")
    }

    fn serial_path(&self) -> Option<String> {
        Some(format!("/dev/cu.usbmodem{}{}", 
            format!("{:x}", self.usb.location_id.unwrap_or(0)).replace("0",""),
            3,
        ))
    }    

    fn openocd_serial(&self) -> Option<String> {
        Some(format!("hla_serial {}", self.usb.serial_number))
    }        
}

pub struct TiIcdiDevice {
    usb: UsbDevice,
}

impl Device for TiIcdiDevice {
    fn usb(&self) -> &UsbDevice {
        &self.usb
    }

    fn device_type(&self) -> Option<&str> {
        Some("TI-ICDI")
    }

    fn serial_path(&self) -> Option<String> {
        Some(format!("/dev/cu.usbmodem{}{}", &self.usb.serial_number[..7], 1))
    }    

    fn openocd_serial(&self) -> Option<String> {
        Some(format!("hla_serial {}", self.usb.serial_number))
    }        
}

pub struct DapLinkDevice {
    usb: UsbDevice,
}

impl Device for DapLinkDevice {
    fn usb(&self) -> &UsbDevice {
        &self.usb
    }

    fn device_type(&self) -> Option<&str> {
        Some("DAPLink")
    }

    fn serial_path(&self) -> Option<String> {
        Some(format!("/dev/cu.usbmodem{}{}", 
            format!("{:x}", self.usb.location_id.unwrap_or(0)).replace("0",""),
            2,
        ))
    }

    fn msd_path(&self) -> Option<PathBuf> {
        // Look in /Volumes/DAPLINK*/ for DETAILS.TXT
        // Look for Unique ID line == serial number
        if let Ok(volumes) = fs::read_dir("/Volumes/") {
            for volume in volumes {                
                if let Ok(volume) = volume {                    
                    //println!("checking {:?} {}", volume.path(), volume.path().to_string_lossy().starts_with("/Volumes/DAPLINK") );
                    if volume.path().to_string_lossy().starts_with("/Volumes/DAPLINK") {                        
                        let details = volume.path().join("DETAILS.TXT");
                        let mut f = fs::File::open(details).expect("Error opening DETAILS.TXT");
                        let mut s = String::new();
                        f.read_to_string(&mut s).expect("Error reading details");
                        if s.contains(&self.usb.serial_number) {
                            return Some(volume.path())
                        }                        
                    }
                }
            }
        }
        None
    }

    fn openocd_serial(&self) -> Option<String> {
        Some(format!("cmsis_dap_serial {}", self.usb.serial_number))
    }    
    
}

pub struct DeviceFilter {
    all: bool,
    device: Option<String>,
}

impl<'a> From<&'a ArgMatches<'a>> for DeviceFilter {
    fn from(other: &ArgMatches) -> DeviceFilter {
        DeviceFilter {
            all: other.is_present("all"),
            device: other.value_of("device").map(String::from)
        }
    }
}

pub fn lookup(usb: UsbDevice) -> Box<Device> {
    match (usb.vendor_id, usb.product_id) {
        (0x0d28, 0x0204) => Box::new(DapLinkDevice { usb: usb }),
        (0x03eb, 0x2157) => Box::new(DapLinkDevice { usb: usb }),
        (0x0483, 0x3748) => Box::new(StLinkV2Device { usb: usb }),
        (0x0483, 0x374b) => Box::new(StLinkV21Device { usb: usb }),
        (0x1366, 0x0101) => Box::new(JLinkDevice { usb: usb }),
        (0x1366, 0x0105) => Box::new(JLinkDevice { usb: usb }),
        (0x1cbe, 0x00fd) => Box::new(TiIcdiDevice { usb: usb }),
        _ => Box::new(UnknownDevice { usb: usb })
    }
}


pub fn enumerate() -> Result<Vec<Box<Device>>> {
    Ok(ioreg::enumerate()?.into_iter().map(lookup).collect())
}

pub fn search(filter: &DeviceFilter) -> Result<Vec<Box<Device>>> {
    Ok(enumerate()?.into_iter().filter(|d| {
        if !filter.all {
            if d.is_unknown() {
                return false
            }
        }

        if let Some(ref device) = filter.device {
            if !d.hash().starts_with(device) {
                return false
            }
        }


        true
    }).collect())
}