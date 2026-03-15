mod bus;
mod cart;
mod cpu;
mod ppu;
mod timer;

use std::fs::File;
use std::io;
use std::io::BufReader;

use chrono::Utc;
use clap::Parser;

use crate::bus::Bus;
use crate::cart::Cartridge;
use crate::cpu::Cpu;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    file: String,
}

fn main() -> io::Result<()> {
    fern::Dispatch::new()
        .format(|out, message, _| out.finish(format_args!("{}", message)))
        .level(log::LevelFilter::Debug)
        .chain(fern::log_file(format!(
            "log/cpu_dump_{}.log",
            Utc::now().format("%d%m%Y_%H%M%S")
        ))?)
        .apply()
        .unwrap();

    let args = Args::parse();
    let file = File::open(args.file)?;
    let mut r = BufReader::new(file);

    let mut cpu = Cpu::default();
    cpu.load_cart(Cartridge::new(&mut r)?);
    cpu.log_state();

    loop {
        cpu.step();
    }
}
