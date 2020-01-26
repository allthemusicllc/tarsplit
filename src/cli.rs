// cli.rs
//
// Copyright (c) 2020 All The Music, LLC
//
// This work is licensed under the Creative Commons Attribution 4.0 International License.
// To view a copy of this license, visit http://creativecommons.org/licenses/by/4.0/ or send
// a letter to Creative Commons, PO Box 1866, Mountain View, CA 94042, USA.

pub struct Cli<'a, 'b> {
    pub app: clap::App<'a, 'b>,
}

impl<'a, 'b> Cli<'a, 'b> {
    fn initialize_parser() -> clap::App<'a, 'b> {
        // Command line app
        clap::App::new("tarsplit")
            .version(env!("CARGO_PKG_VERSION"))
            .author("All The Music, LLC")
            .about("Tool for splitting tar archives into chunks along file boundaries.")
            .arg(clap::Arg::with_name("CHUNK_SIZE")
                .short("c")
                .long("chunk-size")
                .takes_value(true)
                .help("Approximate size of output chunks in bytes (incompatible with NUM_CHUNKS)"))
            .arg(clap::Arg::with_name("NUM_CHUNKS")
                .short("n")
                .long("num-chunks")
                .takes_value(true)
                .help("Number of ouptut chunks (incompatible with CHUNK_SIZE)"))
            .arg(clap::Arg::with_name("PREFIX")
                 .short("p")
                 .long("prefix")
                .takes_value(true)
                .default_value("split")
                .help("Prefix to apply to filename of each output chunk"))
            .arg(clap::Arg::with_name("SOURCE")
                .takes_value(true)
                .required(true)
                .help("Path to source TAR archive"))
            .arg(clap::Arg::with_name("TARGET")
                .takes_value(true)
                .required(true)
                .help("File output path (directory must exist)"))
    }

    pub fn new() -> Cli<'a, 'b> {
        Cli {
            app: Cli::initialize_parser(),
        }
    }

    pub fn run(self) {
        let matches = self.app.get_matches();
        crate::directives::tarsplit(crate::directives::TarsplitDirectiveArgs::from(&matches));
    }
}
