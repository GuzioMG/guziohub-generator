use std::{env::args_os, fs};
use guziohub_generator::*;
use anyhow::{Context, Result, bail};
use walkdir::WalkDir;

fn main() -> Result<()>{
	let path = "test.g.html";
	let file = fs::read_to_string(path).with_context(|| format!("Couldn't read file from {}!", path))?;
	let (meta, lines) = process(&file)?;
	dbg!(meta);
	for line in lines {
		dbg!(line);
	}

	let args: Vec<_> = args_os().collect();
	if let [_cmd, flags@.., src, dest] = args.as_slice() {
		match flags {
			[] => walk_in(src.to_string_lossy().to_string(), dest.to_string_lossy().to_string(), false)?,
			[single] => {
				match single.to_string_lossy().to_string().as_str() {
					"help"|"--help"|"-h" =>{
						print_usage();
						return Ok(print!("   (oh, btw, you don't need to pass extra args when calling -h or --help."))
					}
					"--test-mode" => walk_in(src.to_string_lossy().to_string(), dest.to_string_lossy().to_string(), true)?,
					flag => {
						if flag.starts_with("--") {
							bail!("Unknown flag: {}", flag.strip_prefix("--").unwrap_or(flag));
						} else if flag.starts_with("-") {
							bail!("Unknown flag(s): {}", flag.strip_prefix("-").unwrap_or(flag));
						} else {
							bail!("Got 3 arguments, so assumed the 1st one was a flag - but apparently it wasn't, because it doesn't start with a dash. Please prefix your args with dashes! Or - if you actually didn't mean to put an arg here - this isn't valid usage, try using \"help\"|\"--help\"|\"-h\" for help.");
						}
					}
				}
			}
			_ => arg_fail(flags.len()+2)?
		}
	} else if let [_cmd, flag] = args.as_slice() {
		match flag.to_string_lossy().to_string().as_str() {
			"help"|"--help"|"-h" => return Ok(print_usage()),
			"--test-mode" => bail!("Can't run tests with no src to compare from and no dst to compare to. See Help for information on how to set src/dst."),
			flag => {
				if flag.starts_with("--") {
					bail!("Unknown flag: {}", flag.strip_prefix("--").unwrap_or(flag));
				} else if flag.starts_with("-") {
					bail!("Unknown flag(s): {}", flag.strip_prefix("-").unwrap_or(flag));
				} else {
					bail!("Got 1 argument, so assumed it was a flag - but apparently it wasn't, because it doesn't start with a dash. It wasn't the literal „help” either. This isn't valid usage - try using \"help\"|\"--help\"|\"-h\" for help.");
				}
			}
		}
	} else {
		arg_fail(args.len()-1)?
	}

	return Ok(());
}

fn print_usage() {
	println!("USAGE:  (cmd) [flag] <src> <dst>  |  (cmd) <flag>|<\"help\">");
	println!("ARG: src  ->  The folder in which to search for templates and G-HTML to process, and aux files/assets to copy as-is.");
	println!("ARG: dst  ->  The destination folder. Files may be overwritten (if they differ from processed src) or even removed (if not found in src)!");
	println!("FLAG: --test-mode  ->  Will not change anything in dst, instead checking whether its contents differ in any way from processed src.");
	print!("FLAG: -h | --help  ->  You're reading it, dummy :)");
}

fn arg_fail(count: usize) -> Result<()> {
	bail!("Expected 1-3 arguments (incl. flags), but got {} instead! This isn't valid usage - try using \"help\"|\"--help\"|\"-h\" for help.", count);
}

fn walk_in(src: String, dest: String, test_mode: bool) -> Result<()> {
	for found in WalkDir::new(src).follow_links(true).same_file_system(false) {
		dbg!(found?.path());
	}

	return Ok(());
}