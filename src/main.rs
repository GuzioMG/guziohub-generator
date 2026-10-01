use std::{cmp::max, collections::HashMap, env::args_os, fs, io, ops::AddAssign, path::PathBuf};
use guziohub_generator::*;
use anyhow::{Context, Error, Result, bail, ensure};
use walkdir::WalkDir;

fn main() -> Result<()>{
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

fn walk_in(src: String, dst: String, test_mode: bool) -> Result<()> {
	println!("[1/4] -- READING SRC TREE");
	let mut templates: HashMap<String, io::Result<String>> = HashMap::new();
	let mut assets: HashMap<String, Asset> = HashMap::new();
	let mut sources: Vec<SourcePaths> = Vec::new();
	for found in WalkDir::new(&src).follow_links(true).same_file_system(false) {
		let file = found.with_context(||"File walking error:")?;
		let name = file.file_name().to_string_lossy().to_string();
		let path_raw = file.path();
		match path_raw.to_str() {
			None => bail!("File walking error: Path „{}” contains non-UTF-8 sequences.", path_raw.to_string_lossy()),
			Some(path) => {
				print!("Found {} - it's a", path);

				if name.ends_with(".g.html") {
					println!(" G-HTML file.");
					match path.strip_suffix(".g.html") {
						Some(stripped) => {
							let stripped = stripped.replacen(src.as_str(), &dst, 1);
							let mut ok = stripped.clone();
							let mut err = stripped.clone();
							ok.push_str(".html");
							err.push_str(".autopsy.txt");
							sources.push(SourcePaths { src: path_raw.to_path_buf(), ok, err });
						}
						None => bail!("Evil FS shenanigans seem to be going on - the file NAME ended with .g.html, but the file PATH did not. Refusing to operate in this unstable environment."),
					}
				}

				else if name.starts_with("template_") && name.ends_with(".html") {
					println!(" template.");
					// SAFETY: We just checked that the name starts and ends with template_ and .html
					let name = unsafe{name.strip_circumfix("template_", ".html").unwrap_unchecked()};
					if let Some(_) = templates.insert(name.to_string(), fs::read_to_string(path_raw)) {
						bail!("Template processing error: Found multiple templates called „{}”.", name);
					}
				}

				else {
					println!("n asset.");
					let asset_src;
					let ftype = file.file_type();
					if ftype.is_dir() {
						asset_src = Asset::Directory;
					} else if file.path_is_symlink() && ftype.is_file() {
						asset_src = Asset::Copied(fs::read_link(path_raw).with_context(||format!("File walking error: Couldn't unwrap symlink „{}” to a real path:", path))?.as_path().to_path_buf());
					} else if ftype.is_file() {
						asset_src = Asset::Copied(path_raw.to_path_buf());
					} else {
						bail!("Asset processing error: „{}” is an unsupported asset type (eg. block device / socket).", path);
					}
					
					let placement = path.replacen(src.as_str(), &dst, 1);
					let ctx = format!("Asset processing error: Multiple assets tried to position themselves at „{}”.", placement);
					if let Some(_) = assets.insert(placement, asset_src) {
						bail!(ctx);
					}
				}
			}
		}
	}

	println!("[2/4] -- APPLYING TEMPLATES");
	let mut index: usize = 0;
	let max_index = sources.len();
	for ghtml in sources {
		index+=1;
		print!("Processing {}/{} G-HTML files - {}  ->  ", index, max_index, ghtml.src.as_os_str().to_string_lossy().to_string());
		let file = fs::read_to_string(ghtml.src).with_context(||"Couldn't even get to it due to an IO error:")?;
		match process(&file) {
			Ok((meta, lines)) => {
				print!("Syntax OK!; ");
				match templates.get(meta.get_template()) {
					Some(Ok(template)) => {
						print!("Template FOUND!; ");
						let mut index: usize = 0;
						let max_index: usize = lines.len();
						let mut processed_lines = LineResult::default();
						let mut longest_line: usize = 0;
						for line in lines {
							index+=1;
							line.find_longest(&mut longest_line);
							processed_lines+=LineResult::new(line, index, max_index, processed_lines.time);
						}
						let ctx_good = format!("Will be saved at: {}", ghtml.ok);
						let ctx_bad = format!("Intended save path ({}) already occupied by an asset!", ghtml.ok);
						if let Some(_) = assets.insert(ghtml.ok, Asset::Literal(meta.apply_to_template(&processed_lines.apply_to_template(template)).replace("{{_INTERNAL_LONGEST_CONTENT}}", &longest_line.to_string()))) {
							save_autopsy(&mut assets, Error::msg(ctx_bad), ghtml.err, "Saving IMPOSSIBLE!")?;
						} else {
							println!("{}", ctx_good);
						}
					},
					Some(Err(err)) => bail!("Template processing error: Template „{}” exists, but can't be loaded due to an IO error: {}", meta.get_template(), err),
					None => save_autopsy(&mut assets, Error::msg(format!("templates.get(\"{}\") returned nothing", meta.get_template())), ghtml.err, "Template NOT FOUND!")?,
				}
			},
			Err(err) => {
				save_autopsy(&mut assets, err, ghtml.err, "Syntax ERR!")?;
			}
		}
	}

	println!("[3/4] -- UPDATING DST TREE");
	for found in WalkDir::new(&dst).follow_links(true).same_file_system(false) {
		let file = found.with_context(||"File walking error:")?;
		let path_raw = file.path();
		match path_raw.to_str() {
			None => bail!("File walking error: Path „{}” contains non-UTF-8 sequences.", path_raw.to_string_lossy()),
			Some(path) => {
				let asset = assets.remove(&path.to_string());
				let ftype = file.file_type();
				if ftype.is_dir(){
					match asset {
						Some(Asset::Directory) => println!("Directory at {} already exists; we good.", path),
						Some(_) => bail!("A whole directory has seemingly ceased to be a directory. This is suspiciously unusual - possibly dst was a wrong path? To prevent data loss, any further walking will be paused. If this is intentional, please remove {} manually.", path),
						None => bail!("A whole directory has seemingly gone missing. This is suspiciously unusual - possibly dst was a wrong path? To prevent data loss, any further walking will be paused. If this is intentional, please remove {} manually.", path),
					}
				} else if ftype.is_file() {
					match asset {
						Some(Asset::Directory) | None => {
							print!("{} isn't meant to be a file - removing it... ", path);
							ensure!(!test_mode, "Running in test-mode! No removal allowed.");
							fs::remove_file(path)?;
							if asset.is_some() {
								println!("  ...And replacing with a folder!");
								fs::create_dir_all(path)?;
							} else {
								println!("DONE!");
							}
						}
						Some(Asset::Copied(from)) => {
							if test_mode {
								println!("Found a file at {}. This ROUGHLY matches the expectation of a copied asset at that location, so I'm not doing a diff and assuming it's OK.", path)
							} else {
								println!("Found a file at {}. Coping a static asset onto it...", path);
								fs::copy(from, path_raw)?;
							}
						}
						Some(Asset::Literal(contents)) => {
							if test_mode {
								println!("Found a file at {}. Expected a G-HTML result there, so I'll diff...", path);
								let file = fs::read_to_string(path_raw)?;
								if file != contents {
									bail!("Contents were „{}” instead of the expected „{}”!", file, contents)
								}
							} else {
								println!("Found a file at {}. Writing a G-HTML result onto it...", path);
								fs::write(path_raw, contents)?;
							}
						}
					}
				} else {
					bail!("File walking error: „{}” is an unsupported type (eg. block device / socket).", path);
				}
			}
		}
	}

	println!("[4/4] -- CREATING MISSING ASSETS");
	let mut index: usize = 0;
	let max_index = assets.len();
	for (path, asset) in assets {
		index+=1;
		print!("Creating {}/{} assets - {}  ->  ", index, max_index, path);
		ensure!(!test_mode, "Running in test-mode! No creation allowed.");
		match asset {
			Asset::Directory => {
				println!("Creating dir(s)...",);
				fs::create_dir_all(path)?;
			},
			Asset::Copied(to) =>  {
				println!("Coping a static asset...",);
				fs::copy(path, to)?;
			},
			Asset::Literal(contents) =>  {
				println!("Writing a G-HTML result...",);
				fs::write(path, contents)?;
			},
		}
	}

	return Ok(());
}

fn save_autopsy(assets: &mut HashMap<String, Asset>, err: Error, path: String, msg: &str) -> Result<()>{
	print!("{}, will save an autopsy; ", msg);
	let ctx_good = format!("Autopsy will be saved at: {}", path);
	let ctx_bad = format!("Autopsy path ({}) already occupied by an asset!", path);
	if let Some(_) = assets.insert(path, Asset::Literal(err.to_string())) {
		bail!(ctx_bad);
	} else {
		return Ok(println!("{}", ctx_good));
	}
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Asset {
	#[default] Directory,
	Copied(PathBuf),
	Literal(String),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SourcePaths {
	src: PathBuf,
	ok: String,
	err: String
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LineResult {
	html: String,
	css: String,
	time: usize
}

impl AddAssign for LineResult {
	fn add_assign(&mut self, rhs: Self) {
		self.html.push_str(&rhs.html);
		self.css.push_str(&rhs.css);
		self.time+=rhs.time;
	}
}

impl ApplyToTemplate for LineResult {
	fn apply_to_template(&self, template: &String) -> String {
		return template
		.replace("{{PAGE_CONTENT}}", &self.html)
		.replace("/*Slot for auto-generated CSS*/", &self.css);
	}
}

impl LineResult {
	fn new(line: StringGaslitAboutItsLength, index: usize, last_index: usize, seconds: usize) -> Self {
		let (text, length) = line.unwrap();

		// HTML PART
		let mut prefix = format!("\n            <br><p class=\"{{{{_INTERNAL_DEFAULT_CLASS}}}}\">&nbsp;$&nbsp;</p><p class=\"{{{{_INTERNAL_DEFAULT_CLASS}}}} typing-animator-moving\">");
		if index == 1 {
			prefix = prefix.replace("<br>", "");
		} else {
			prefix = prefix.replace("&nbsp;$", "");
		}
		let html = format!("{}{}</p><p class=\"{{{{_INTERNAL_DEFAULT_CLASS}}}} typing-animator-moving typing-animator-blinking\">_</p>", prefix, text);

		// CSS PART
		const CPS:usize = 25; //Chars. per second calculated from: * Average reading speed is 200-300WPM, going with the higher-end and turning to seconds we get 5WPS. Then, average chars in a word is 4.7=~5. Overall, 5wps*5cpw = 25cps.
		let local_prefix = "\n            .typing-animator:nth-child(";
		let suffix = ";\n            }\n            ";
		let mut prefix = format!("\n            {}", local_prefix);
		let mut time_typing = max(length/CPS, 1);
		let mut time_waiting: usize = 0;
		let time = match index {
			1 => {
				time_typing = 3;
				time_waiting = 1;
				prefix = format!("/*Autogenerated typing animation - see: github.com/GuzioMG/guziohub-generator for details*/\n            \n            body {{\n                --content-length: {{{{_INTERNAL_LONGEST_CONTENT}}}}{}{}", suffix, prefix);
				1
			},
			2 => {
				time_waiting = 1;
				0
			},
			_ => if index == last_index {
				3
			} else {
				0
			}
		}+time_waiting+time_typing;
		let css = format!("{}{}) {{\n                animation: type {}s steps({}) {}s 1 normal forwards{}{}{}) {{\n                animation: blinker 0.5s steps(1) {}s {} normal forwards {}", prefix, 2+4*(index-1), time_typing, length, seconds+time_waiting, suffix, local_prefix, 3+4*(index-1), seconds, time*2, suffix);

		return LineResult { html, css, time };
	}
}