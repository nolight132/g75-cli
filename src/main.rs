use clap::{Parser, Subcommand};
use std::{
    fs::{self, File, OpenOptions},
    os::fd::AsRawFd,
    process,
};

const HID_ID: &str = "HID_ID=0003:0000258A:0000010C";
/// Descriptor bytes declaring the 519 byte feature report that only the
/// configuration interface has.
const CONFIG_REPORT: &[u8] = &[0x96, 0x07, 0x02, 0xb1, 0x02];
const REPORT_ID: u8 = 6;
const REPORT_SIZE: usize = 520;

const READ_PERFORMANCE: u8 = 0x84;
const READ_LIGHT_COLOR: u8 = 0x8a;
const WRITE_LIGHT_COLOR: u8 = 0x0a;

const LIGHT_MODE_OFFSET: usize = 18;
const COLOR_TABLE_OFFSET: usize = 8;
const COLOR_SLOT_SIZE: usize = 21;
const COLOR_MODES: &[u8] = &[1, 2, 3, 4, 5, 7, 8, 10, 11, 12, 13];
const TRAILER_OFFSET: usize = 514;
const TRAILER: [u8; 2] = [0x5a, 0xa5];

const AFTER_HELP: &str = "\
Examples:
  g75-cli color
  g75-cli color eb1c34";

/// Control the lighting of a wired MCHOSE G75 Pro.
#[derive(Parser)]
#[command(version, after_help = AFTER_HELP)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the colour of the current light mode, or set it
    Color {
        /// New colour as RRGGBB, with or without a leading #
        #[arg(value_parser = parse_color)]
        color: Option<[u8; 3]>,
    },
}

fn parse_color(text: &str) -> Result<[u8; 3], String> {
    let hex = text.strip_prefix('#').unwrap_or(text);
    let invalid = || format!("'{text}' is not a RRGGBB colour");
    if hex.len() != 6 || !hex.is_ascii() {
        return Err(invalid());
    }
    let mut color = [0; 3];
    for (i, byte) in color.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|_| invalid())?;
    }
    Ok(color)
}

fn hex(color: &[u8]) -> String {
    color.iter().map(|b| format!("{b:02x}")).collect()
}

/// Prints `message` as an error and exits with status 1.
fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("error: {message}");
    process::exit(1);
}

fn main() {
    let Command::Color { color } = Cli::parse().command;

    let device = open_keyboard();
    let mode = read_block(&device, READ_PERFORMANCE, 0x80)[LIGHT_MODE_OFFSET];
    if !COLOR_MODES.contains(&mode) {
        fail(format!("light mode {mode} has no single colour"));
    }

    let mut block = read_block(&device, READ_LIGHT_COLOR, 0x200);
    if block[TRAILER_OFFSET..TRAILER_OFFSET + 2] != TRAILER {
        fail("colour block has an unknown layout, refusing to write");
    }
    let slot = COLOR_TABLE_OFFSET + COLOR_SLOT_SIZE * mode as usize;
    let current = &block[slot..slot + 3];

    let Some(color) = color else {
        println!("{}", hex(current));
        return;
    };

    // Every write lands in the keyboard's flash, so skip the ones that change nothing.
    if current != color {
        block[1] = WRITE_LIGHT_COLOR;
        block[slot..slot + 3].copy_from_slice(&color);
        set_feature(&device, &block);
        let written = read_block(&device, READ_LIGHT_COLOR, 0x200);
        if written[slot..slot + 3] != color {
            fail(format!(
                "keyboard reports {} after writing {}",
                hex(&written[slot..slot + 3]),
                hex(&color)
            ));
        }
    }
    println!("G75 Pro: color {}", hex(&color));
}

/// Opens the hidraw node of the keyboard's configuration interface. The
/// keyboard exposes two nodes and only one declares the configuration report.
fn open_keyboard() -> File {
    let mut nodes: Vec<_> = fs::read_dir("/sys/class/hidraw")
        .map(|dir| dir.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    nodes.sort();

    for node in nodes {
        let uevent = fs::read_to_string(node.join("device/uevent")).unwrap_or_default();
        if !uevent.lines().any(|line| line == HID_ID) {
            continue;
        }
        let descriptor = fs::read(node.join("device/report_descriptor")).unwrap_or_default();
        if !descriptor
            .windows(CONFIG_REPORT.len())
            .any(|w| w == CONFIG_REPORT)
        {
            continue;
        }
        let path = format!("/dev/{}", node.file_name().unwrap().to_string_lossy());
        return OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap_or_else(|e| {
                fail(format!(
                    "cannot open {path}: {e} (is the udev rule installed?)"
                ))
            });
    }
    fail("no wired MCHOSE G75 Pro found");
}

/// Builds a read-write hidraw ioctl request, `HIDIOCSFEATURE` for 0x06 and
/// `HIDIOCGFEATURE` for 0x07.
fn hid_ioctl(nr: u32) -> u32 {
    (3 << 30) | ((REPORT_SIZE as u32) << 16) | ((b'H' as u32) << 8) | nr
}

fn set_feature(device: &File, report: &[u8; REPORT_SIZE]) {
    // SAFETY: the buffer is REPORT_SIZE bytes long, matching the size encoded in the request.
    let result = unsafe { libc::ioctl(device.as_raw_fd(), hid_ioctl(0x06) as _, report.as_ptr()) };
    if result < 0 {
        fail(format!(
            "cannot send report: {}",
            std::io::Error::last_os_error()
        ));
    }
}

/// Reads one configuration block by sending its read command and fetching the
/// report back. Exits unless the keyboard echoes the command.
fn read_block(device: &File, command: u8, length: u16) -> [u8; REPORT_SIZE] {
    let mut report = [0; REPORT_SIZE];
    let [low, high] = length.to_le_bytes();
    report[..8].copy_from_slice(&[REPORT_ID, command, 0, 0, 1, 0, low, high]);
    set_feature(device, &report);

    let mut reply = [0; REPORT_SIZE];
    reply[0] = REPORT_ID;
    // SAFETY: the buffer is REPORT_SIZE bytes long, matching the size encoded in the request.
    let result =
        unsafe { libc::ioctl(device.as_raw_fd(), hid_ioctl(0x07) as _, reply.as_mut_ptr()) };
    if result < 0 {
        fail(format!(
            "cannot read report: {}",
            std::io::Error::last_os_error()
        ));
    }
    if reply[1] != command {
        fail(format!(
            "unexpected reply 0x{:02x} to command 0x{command:02x}",
            reply[1]
        ));
    }
    reply
}
