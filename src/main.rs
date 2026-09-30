use std::fs;
use guziohub_generator::*;
use anyhow::{Context, Result};

fn main() -> Result<()>{
	let path = "test.html";
	let file = fs::read_to_string(path).with_context(|| format!("Couldn't read file from {}!", path))?;
	let (meta, lines) = process(&file)?;
	dbg!(meta);
	for line in lines {
		dbg!(line);
	}
	return Ok(());
}