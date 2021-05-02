extern crate vst;

use std::path::Path;
use std::sync::{Arc, Mutex};

use self::vst::host::PluginInstance;
use vst::host::{Host, PluginLoader};
use vst::plugin::Plugin;

#[allow(dead_code)]
pub struct VSTHost {
    pub instance: PluginInstance,
}

struct SimpleHost;

impl Host for SimpleHost {
    fn automate(&self, index: i32, value: f32) {
        println!("Parameter {} had its value changed to {}", index, value);
    }
}

impl VSTHost {
    pub fn load(filename: &str) -> VSTHost {
        let host = Arc::new(Mutex::new(SimpleHost));

        let path = Path::new(filename);
        println!("Loading {}...", path.to_str().unwrap());

        // Load the plugin
        let mut loader = PluginLoader::load(path, Arc::clone(&host))
            .unwrap_or_else(|e| panic!("Failed to load plugin: {}", e.to_string()));

        let mut result = VSTHost {
            instance: loader.instance().unwrap(),
        };

        // Get the plugin information
        let info = result.instance.get_info();

        println!(
            "Loaded '{}':\n\t\
         Vendor: {}\n\t\
         Presets: {}\n\t\
         Parameters: {}\n\t\
         VST ID: {}\n\t\
         Version: {}\n\t\
         Initial Delay: {} samples",
            info.name,
            info.vendor,
            info.presets,
            info.parameters,
            info.unique_id,
            info.version,
            info.initial_delay
        );

        // Initialize the instance
        result.instance.init();
        println!("Initialized instance!");
        result
    }
}

//fn main() {
//    let args: Vec<String> = env::args().collect();
//    if args.len() < 2 {
//        println!("usage: simple_host path/to/vst");
//        process::exit(1);
//    }
//
//    let path = Path::new(&args[1]);
//
//    // Create the host
//    let host = Arc::new(Mutex::new(VSTHost));
//
//    println!("Loading {}...", path.to_str().unwrap());
//
//    // Load the plugin
//    let mut loader = PluginLoader::load(path, Arc::clone(&host))
//        .unwrap_or_else(|e| panic!("Failed to load plugin: {}", e.description()));
//
//    // Create an instance of the plugin
//    let mut instance = loader.instance().unwrap();
//
//    // Get the plugin information
//    let info = instance.get_info();
//
//    println!(
//        "Loaded '{}':\n\t\
//         Vendor: {}\n\t\
//         Presets: {}\n\t\
//         Parameters: {}\n\t\
//         VST ID: {}\n\t\
//         Version: {}\n\t\
//         Initial Delay: {} samples",
//        info.name, info.vendor, info.presets, info.parameters, info.unique_id, info.version, info.initial_delay
//    );
//
//    // Initialize the instance
//    instance.init();
//    println!("Initialized instance!");
//
//    println!("Closing instance...");
//    // Close the instance. This is not necessary as the instance is shut down when
//    // it is dropped as it goes out of scope.
//    // drop(instance);
//}
