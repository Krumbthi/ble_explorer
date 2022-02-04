use std::thread;
use std::time::Duration;
use std::sync::Arc;

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
use sixtyfps::{Timer, TimerMode, quit_event_loop, SharedString};
sixtyfps::include_modules!();

// concurrency
use futures::future::{Fuse, FusedFuture, FutureExt};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

struct PrinterQueueData {
    data: Rc<sixtyfps::VecModel<PrinterQueueItem>>,
    print_progress_timer: sixtyfps::Timer,
}

fn main() {
    // set up logging
    let env = Env::filter_or(Env::default(), "APP_LOG_LEVEL", "debug")
        .write_style_or("APP_LOG_STYLE", "always");    
    env_logger::init_from_env(env);

    // SixtyFPS UI
    let main_window = MainWindow::new();

    // instantiate a timer object
    let timer = Arc::new(Timer::default());
    
    let timer_copy = timer.clone();
    let main_window_weak = main_window.as_weak();

    let printer_queue = Rc::new(PrinterQueueData {
        data: Rc::new(sixtyfps::VecModel::from(default_queue)),
        print_progress_timer: Default::default(),
    });

    main_window.on_run_bluetooth(move || {
        let window_handle = main_window_weak.unwrap();
        
        //timer_copy.start(TimerMode::SingleShot, std::time::Duration::from_millis(20), move || {
        let bt_session = &BluetoothSession::create_session(None).unwrap();
        let adapter: BluetoothAdapter = BluetoothAdapter::init(bt_session).unwrap();
            
        if let Err(_error) = adapter.set_powered(true) {
            error!("Failed to power adapter");
            panic!("Failed to power adapter");
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
            
        if let Err(e) = device.connect(10000) {
            error!("Failed to connect {:?}: {:?}", device.get_id(), e);
        } else {
            window_handle.set_info(SharedString::from("Status: Notification stoped"));
            // We need to wait a bit after calling connect to safely
            // get the gatt services
            window_handle.set_info(SharedString::from("Status: Connecting ..."));
            thread::sleep(Duration::from_millis(5000));
                    
            // find the gatt service
            let char_path = explore_device::find_characteristic_path(bt_session, &device);
            let temp_humidity = BluetoothGATTCharacteristic::new(bt_session, char_path);
            
            // let timer = Timer::default();
            temp_humidity.start_notify().unwrap();
            window_handle.set_info(SharedString::from("Status: Triggered notification"));
            
            // timer_copy.start(TimerMode::Repeated, std::time::Duration::from_millis(20), move || {
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
                    let t = v["Rumqtt"]["Temperature"].as_f64().unwrap();
                    let h = v["Rumqtt"]["Humidity"].as_f64().unwrap();
                    let p = v["Rumqtt"]["Pressure"].as_f64().unwrap();
                    
                    info!("Temperature: {}ºC \tHumidity: {}% \tPressure: {}Pa", v["Rumqtt"]["Temperature"], v["Rumqtt"]["Humidity"], v["Rumqtt"]["Pressure"]);
                    main_window_weak.upgrade_in_event_loop(move |main_window| {main_window.set_temperature(SharedString::from(format!("{} ºC", t))) });
                    
                    window_handle.set_humidity(SharedString::from(format!("{} %", h)));
                    window_handle.set_pressure(SharedString::from(format!("{} kPa", p)));
                    //window_handle.set_temperature(SharedString::from(format!("{} ºC", t)));
                }
            }
        }
    });

    let timer_copy = timer.clone();
    let main_window_weak = main_window.as_weak();
    main_window.on_stop_bluetooth(move || {
        let window_handle = main_window_weak.unwrap();
        if timer_copy.running() {
            debug!("Stopping notification");
            timer_copy.stop();
        } else {
            return;
        }
        window_handle.set_info(SharedString::from("Status: Notification stoped"));
    });

    main_window.on_quit(move || {
        debug!("exit notification");
        quit_event_loop();
    }); 

    main_window.run();
    
}
