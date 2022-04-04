extern crate clap;
extern crate colored;
extern crate nix;
extern crate rand;

use std::{io, process::Command, str::FromStr};

use clap::{Parser, Subcommand};
use colored::Colorize;
use nix::{ifaddrs::getifaddrs, sys::socket::SockAddr};
use rand::random;

/// A simple  MAC address utility
#[derive(Parser, Debug)]
#[clap(author, version, about, arg_required_else_help(true), long_about = None)]
struct Args {
	/// Set MAC address
	#[clap(subcommand)]
	set: Option<SubCmds>,

	/// Generate a random MAC address
	#[clap(short, long)]
	random: bool,

	/// Print current MAC address
	#[clap(short, long)]
	current: bool,
}

#[derive(Debug, Subcommand)]
enum SubCmds {
	Set {
		/// New MAC address to use
		#[clap(short, long)]
		address: Option<String>,

		/// Interface to use (name)
		#[clap(short, long)]
		interface: Option<String>,

		/// Use a random MAC address
		#[clap(short, long)]
		random: bool,
	},
}

enum MacParseError {
	/// Parsing of the MAC address contained an invalid digit.
	InvalidDigit,
	/// The MAC address did not have the correct length.
	InvalidLength,
}

impl std::fmt::Display for MacParseError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		f.write_str(match *self {
			MacParseError::InvalidDigit => "invalid digit",
			MacParseError::InvalidLength => "invalid length",
		})
	}
}

struct MacAddr {
	bytes: [u8; 6],
}

impl MacAddr {
	fn new(bytes: [u8; 6]) -> MacAddr { MacAddr { bytes } }
}

impl std::fmt::Display for MacAddr {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}:{:<02X}",
			self.bytes[0],
			self.bytes[1],
			self.bytes[2],
			self.bytes[3],
			self.bytes[4],
			self.bytes[5]
		)?;
		Ok(())
	}
}

impl std::str::FromStr for MacAddr {
	type Err = MacParseError;

	fn from_str(input: &str) -> Result<Self, Self::Err> {
		let mut array = [0u8; 6];

		let mut nth = 0;
		for byte in input.split(|c| c == ':' || c == '-') {
			if nth == 6 {
				return Err(MacParseError::InvalidLength);
			}

			array[nth] = u8::from_str_radix(byte, 16).map_err(|_| MacParseError::InvalidDigit)?;

			nth += 1;
		}

		if nth != 6 {
			return Err(MacParseError::InvalidLength);
		}

		Ok(MacAddr::new(array))
	}
}

/// Check if a wireless interface exists (given the name)
fn inter_exists(inter: &str) -> io::Result<bool> {
	let ifiter = getifaddrs()?;
	for interface in ifiter {
		if interface.interface_name == inter {
			return Ok(true);
		}
	}
	Ok(false)
}

/// Get current network info (interface, address)
fn get_info(name: Option<&str>) -> io::Result<Option<(String, MacAddr)>> {
	let ifiter = getifaddrs()?;

	for interface in ifiter {
		if let Some(address) = interface.address {
			if let SockAddr::Link(link) = address {
				let bytes = link.addr();

				if let Some(name) = name {
					if interface.interface_name == name {
						return Ok(Some((name.to_string(), MacAddr { bytes })));
					}
				} else if bytes.iter().any(|&x| x != 0) {
					return Ok(Some((
						interface.interface_name.to_string(),
						MacAddr { bytes },
					)));
				}
			}
		}
	}
	Ok(None)
}

/// Generate a valid MAC address
fn new_addr() -> MacAddr {
	let mut addr: MacAddr = MacAddr { bytes: [0; 6] };
	for i in 0..6 {
		addr.bytes[i] = random::<u8>();
	}
	addr.bytes[0] &= 0xfe; // clear multicast bit
	addr.bytes[0] |= 0x02; // set local assignment bit (IEEE802)
	addr
}

/// Set MAC address, given an interface name and a MAC address
fn set_addr(inter: &str, addr: MacAddr) -> io::Result<()> {
	// sudo ip link set [interface] down
	Command::new("sudo")
		.args(["ip", "link", "set", inter, "down"])
		.output()?;
	// sudo ip link set [interface] address [MAC address]
	Command::new("sudo")
		.args(["ip", "link", "set", inter, "address", &addr.to_string()])
		.output()?;
	// sudo ip link set [interface] up
	Command::new("sudo")
		.args(["ip", "link", "set", inter, "up"])
		.output()?;
	println!(
		"Set MAC address ({}) to {}",
		inter,
		addr.to_string().green().bold()
	);
	Ok(())
}

fn main() -> io::Result<()> {
	let args = Args::parse();
	// Print current MAC
	if args.current {
		if let Some((current_inter, addr)) = get_info(None).map_err(|e| {
			println!("Failed to get MAC and interface info: {}", e);
			e
		})? {
			println!(
				"Your current MAC address ({}): {}",
				current_inter,
				addr.to_string().green().bold()
			);
			return Ok(());
		} else {
			println!("{}", "No MAC address found :(".red().bold());
		}
	}
	// Generate a random MAC address
	else if args.random {
		let addr = new_addr();
		println!("Random MAC address: {}", addr.to_string().green().bold());
	}
	// Set MAC
	else if let Some(SubCmds::Set {
		address,
		interface,
		random,
	}) = args.set
	{
		// If only the interface option is passed
		if interface.is_some() && address.is_none() && !random {
			println!("{}", "You can't just pass an interface, use -r for a random address or use -a to specify an address".red());
		}
		// Generate and set a random MAC
		else if random {
			// Notify the user than -r takes precedence over -i
			if address.is_some() {
				println!(
					"{}",
					"Using a random MAC address even though the '--address' flag was specified"
						.yellow()
				);
			}
			// Generate MAC address
			let new_addr = new_addr();
			// Check for provided interface
			if let Some(inter) = interface {
				if inter_exists(&inter)? {
					set_addr(&inter, new_addr)?;
				} else {
					println!("Interface doesn't exist: '{}'", inter.red().bold());
				}
			}
			// If no valid interface is provided, fall back to the first valid one
			else {
				println!(
					"{}",
					"No interface provided, using the first valid interface".yellow()
				);
				if let Some((inter, _)) = get_info(None).map_err(|e| {
					println!("Failed to get interface info: {}", e);
					e
				})? {
					set_addr(&inter, new_addr)?;
				} else {
					unreachable!("Issue getting interface info");
				}
			}
		}
		// Set a given MAC
		else if let Some(addr) = address {
			// Get the address
			let addr: MacAddr = MacAddr::from_str(&addr).map_err(|e| {
				let e = format!("Invalid MAC address: {e}");
				println!("Not a valid MAC address: '{}'", addr.red().bold());
				io::Error::new(io::ErrorKind::InvalidInput, e)
			})?;
			// Use interface provided
			if let Some(inter) = interface {
				// Set address if valid interface
				if inter_exists(&inter)? {
					set_addr(&inter, addr)?;
				} else {
					println!("Interface doesn't exist: '{}'", inter.red().bold());
				}
			}
			// No interface provided
			else {
				println!(
					"{}",
					"No interface provided, using the first valid interface".yellow()
				);
				// Get first valid interface
				if let Some((inter, _)) = get_info(None).map_err(|e| {
					println!("Failed to get interface information: {}", e);
					e
				})? {
					set_addr(&inter, addr)?;
				} else {
					unreachable!("Issue getting interface information");
				}
			}
		}
	} else {
		unreachable!("You shouldn't be here");
	}
	Ok(())
}