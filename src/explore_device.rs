use std::str;
use regex::Regex;
use lazy_static::lazy_static;
use blurz::{
    BluetoothDevice,
    BluetoothSession,
    BluetoothGATTService,
    BluetoothGATTCharacteristic,
    BluetoothGATTDescriptor
};
use log::{info, debug, error};

const UUID_REGEX: &str = r"([0-9a-f]{8})-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}";

pub fn explore_gatt_profile(session: &BluetoothSession, device: &BluetoothDevice) { // -> Result<String, Box<dyn Error>> {
    info!("{}", device.get_name().unwrap());

    let services_list = match device.get_gatt_services() {
        Ok(services) => services,
        Err(e) => {
            error!("Failed to get services: {:?}", e);
            return
        }
    };

    lazy_static! {
        static ref RE: Regex = Regex::new(UUID_REGEX).unwrap();
    }

    for service_path in services_list {
        let service = BluetoothGATTService::new(session, service_path.clone());
        let uuid = service.get_uuid().unwrap();
        let assigned_number = RE.captures(&uuid).unwrap().get(1).map_or("", |m| m.as_str());

        debug!("Service UUID: {:?} Assigned Number: 0x{:?}", uuid, assigned_number);
        let characteristics = match service.get_gatt_characteristics() {
            Ok(characteristics) => characteristics,
            Err(e) => {
                error!("Failed to get characteristics: {:?}", e);
                return
            }
        };
         
        for characteristic_path in characteristics {
            explore_gatt_characteristic(session, characteristic_path);
            //get_characteristic_path(session, characteristic_path);
        }
        debug!("-----------------------------------");
        
    }
    debug!("##########################################");
}

fn explore_gatt_characteristic(session: &BluetoothSession, characteristic_path: String) {
    let characteristic = BluetoothGATTCharacteristic::new(session, characteristic_path.clone());
    lazy_static! {
        static ref RE: Regex = Regex::new(UUID_REGEX).unwrap();
    }
    let uuid = characteristic.get_uuid().unwrap();
    let assigned_number = RE.captures(&uuid).unwrap().get(1).map_or("", |m| m.as_str());

    let flags = characteristic.get_flags().unwrap();
    debug!(" Characteristic ID: {:?} Assigned Number: 0x{:?} Flags: {:?}", characteristic_path, assigned_number, flags);

    let descriptors = match characteristic.get_gatt_descriptors() {
        Ok(descriptors) => descriptors,
        Err(e) => {
            error!("Failed to get descriptors: {:?}", e);
            return
        }
    };
    
    for descriptor_path in descriptors {
        explore_gatt_descriptor(&session, descriptor_path);
    }
}

fn explore_gatt_descriptor(session: &BluetoothSession, descriptor_path: String) {
    let descriptor = BluetoothGATTDescriptor::new(session, descriptor_path);
    lazy_static! {
        static ref RE: Regex = Regex::new(UUID_REGEX).unwrap();
    }
    let uuid = descriptor.get_uuid().unwrap();
    let assigned_number = RE.captures(&uuid).unwrap().get(1).map_or("", |m| m.as_str());

    let value = descriptor.read_value(None).unwrap();
    let value = match &assigned_number[4..] {
        "2901" => str::from_utf8(&value).unwrap().to_string(),
        _ => format!("{:?}", value)
    };

    debug!("    Descriptor Assigned Number: 0x{:?} Read Value: {:?}", assigned_number, value);
}

pub fn find_characteristic_path(session: &BluetoothSession, device: &BluetoothDevice) -> String {
    let services_list = match device.get_gatt_services() {
        Ok(services) => services,
        Err(e) => {
            error!("Failed to get services: {:?}", e);
            vec![format!("Error: {:?}", e)]
        }
    };

    lazy_static! {
        static ref RE: Regex = Regex::new(UUID_REGEX).unwrap();
    }
    
    for service_path in services_list {
        let service = BluetoothGATTService::new(session, service_path.clone());
        let uuid = service.get_uuid().unwrap();
        let assigned_number = RE.captures(&uuid).unwrap().get(5).map_or("", |m| m.as_str());

        debug!("Service UUID: {:?} Assigned Number: 0x{:?}", uuid, assigned_number);
        let characteristics = match service.get_gatt_characteristics() {
            Ok(characteristics) => characteristics,
            Err(e) => {
                error!("Failed to get characteristics: {:?}", e);
                vec![format!("Error: {:?}", e)]
            }
        };
        
        for characteristic_path in characteristics {
            let characteristic = BluetoothGATTCharacteristic::new(session, characteristic_path.clone());
            let flags = characteristic.get_flags().unwrap();

            if flags.contains(&String::from("notify")) {
                debug!(" Characteristic ID: {:?} Assigned Number: 0x{:?} Flags: {:?}", characteristic_path, assigned_number, flags);
                return characteristic_path;
            } else {
                //format!("")
                continue;
            }
        }
        ()
    }
    format!("")
}

fn get_characteristic_path(session: &BluetoothSession, characteristic_path: String) -> String {
    let characteristic = BluetoothGATTCharacteristic::new(session, characteristic_path.clone());
    lazy_static! {
        static ref RE: Regex = Regex::new(UUID_REGEX).unwrap();
    }
    let uuid = characteristic.get_uuid().unwrap();
    let assigned_number = RE.captures(&uuid).unwrap().get(1).map_or("", |m| m.as_str());

    let flags = characteristic.get_flags().unwrap();
    debug!(" Characteristic ID: {:?} Assigned Number: 0x{:?} Flags: {:?}", characteristic_path, assigned_number, flags);

    if flags.contains(&String::from("notify")) {
        /*let descriptors = match characteristic.get_gatt_descriptors() {
            Ok(descriptors) => descriptors,
            Err(e) => {
                error!("Failed to get descriptors: {:?}", e);
            }
        };
        for descriptor_path in descriptors {
            explore_gatt_descriptor(&session, descriptor_path);
        }*/
        characteristic_path
    } else {
        String::from("")
    }
}
