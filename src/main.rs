use std::thread;
use std::time::Duration;
use std::process;

use blurz::{
    BluetoothGATTCharacteristic,
    BluetoothEvent,
    BluetoothSession,
    BluetoothAdapter,
    BluetoothDiscoverySession,
    BluetoothDevice
};
use env_logger::Env;
use log::{info, debug, error};
use serde_json::Value;

mod explore_device;

// GUI stuff
use sixtyfps::{Timer, TimerMode, quit_event_loop};
sixtyfps::include_modules!();

static mut Run: bool = false;

struct Application {
    run_flag: bool,
    temp: f64,
    hum: f64,
}

impl Application {
    pub fn ble_quit() {
        debug!("See ya");
        //process::exit(0);
        quit_event_loop();
    }

    pub fn ble_stop(&self, run: bool) {
        self.run_flag = run;
        debug!("Stopping notification");
        temp_humidity.stop_notify().unwrap();
    }

    pub fn ble_start(&self, run: bool) {
        self.run_flag = true;
        // self.proc_timer.start(TimerMode::Repeated, Duration::from_secs(1), Application::process)
    }


    fn process() {
        let bt_session = &BluetoothSession::create_session(None).unwrap();
        let adapter: BluetoothAdapter = BluetoothAdapter::init(bt_session).unwrap();
        if let Err(_error) = adapter.set_powered(true) {
            error!("Failed to power adapter");
            panic!("Failed to power adapter")
        }

        let discover_session = BluetoothDiscoverySession::create_session(&bt_session, adapter.get_id()).unwrap();
        if let Err(_error) = discover_session.start_discovery() {
            error!("Failed to start discovery");
            panic!("Failed to start discovery");
        }
        let device_list = adapter.get_device_list().unwrap();

        discover_session.stop_discovery().unwrap();

        info!("{:?} devices found", device_list.len());

        for device_path in device_list {
            let device = BluetoothDevice::new(bt_session, device_path.to_string());
            debug!("Device: {:?} Name: {:?}", device_path, device.get_name().ok());
        }

        debug!("----------------");
        
        let device = BluetoothDevice::new(bt_session, String::from("/org/bluez/hci0/dev_E4_5F_01_55_13_1A"));
        //let device = BluetoothDevice::new(bt_session, String::from("/org/bluez/hci0/dev_B0_A4_60_7C_D1_FB"));

        if let Err(e) = device.connect(10000) {
            error!("Failed to connect {:?}: {:?}", device.get_id(), e);
        } else {
            // We need to wait a bit after calling connect to safely
            // get the gatt services
            thread::sleep(Duration::from_millis(5000));
                
            explore_device::explore_gatt_profile(bt_session, &device);    
            let temp_humidity = BluetoothGATTCharacteristic::new(bt_session, device.get_id() + "/service0023/char0024");
            temp_humidity.start_notify().unwrap();
        
            debug!("------- READINGS -------");        
            loop {        
                for event in BluetoothSession::create_session(None).unwrap().incoming(1000).map(BluetoothEvent::from) {
                    if event.is_none() {
                        continue;
                    }

                    let value = match event.clone().unwrap() {
                        BluetoothEvent::Value {object_path : _, value} => value,
                        _ => continue
                    };
                    let data = std::str::from_utf8(&*value).unwrap();
                    // Parse the string of data into serde_json::Value.
                    let v: Value = serde_json::from_str(data).unwrap();
                    info!("Temperature: {}ºC \tHumidity: {}% \tPressure: {}Pa", v["Rumqtt"]["Temperature"], v["Rumqtt"]["Humidity"], v["Rumqtt"]["Pressure"]);
                }
            }
        }
    }
}

fn main() {
    // set up logging
    let env = Env::filter_or(Env::default(), "APP_LOG_LEVEL", "debug")
        .write_style_or("APP_LOG_STYLE", "always");    
    env_logger::init_from_env(env);

    let app = Application {
        //proc_timer: Timer::default(),
        run_flag: false,
        temp: 0.0,
        hum: 0.0,
    };

    // SixtyFPS UI
    let main_window = MainWindow::new();
    main_window.on_run_bluetooth(app.ble_start);
    main_window.on_stop_bluetooth(app.ble_stop);
    main_window.on_quit(Application::ble_quit);

    main_window.run();
    
}
