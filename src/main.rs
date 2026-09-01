use std::fs;
use guziohub_generator::*;
use anyhow::{Context, Result};

fn main() -> Result<()>{
	let path = "test.html";
	println!("{}", process(&fs::read_to_string(path).with_context(|| format!("Couldn't read file from {}!", path))?)?);
	return Ok(());
}