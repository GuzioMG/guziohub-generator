use anyhow::{Result, Context, bail, ensure};

pub fn process(filecontent: &String) -> Result<String>{
	let lines = filecontent.lines().collect::<Vec<&str>>();
	let meta = Metadata::new(lines.as_slice())?;
	dbg!(&meta.0);
	return Ok(meta.1);
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Metadata<'metadata_fields>{
	lang: &'metadata_fields str,
	canonical: &'metadata_fields str,
	title: &'metadata_fields str,
	header: &'metadata_fields str,
	description: &'metadata_fields str,
}

impl<'output> Metadata<'output> {
	fn new<'inputs>(lines: &[&'inputs str]) -> Result<(Self, String)>
		where 'inputs: 'output
	{
		if let [doctype, header, content @ .., closing_tag] = lines {
			ensure!(doctype.starts_with("<!DOCTYPE ghtml-v1.0 \"") && doctype.ends_with("\">"), "Invalid G-HTML structure: Invalid doctype! Expected the 1st line to start with „<!DOCTYPE ghtml-v1.0 \"” and end with „\">”, but got „{}” instead.", doctype);
			ensure!(closing_tag.to_string() == "</html>", "Invalid G-HTML structure: No valid closing tag! Expected the last line to be „</html>”, but got „{}” instead.", closing_tag);
			
			let (lang, next_header_segment) = header.strip_prefix("<html flavor=\"ghtml\" lang=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to start with „<html flavor=\"ghtml\" lang=\"”, but got „{}” instead.", header))?
				.split_once("\" canonical=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" canonical=\"” after the the lang param, but got „{}” instead.", header))?;
			let (canonical, next_header_segment) = next_header_segment.split_once("\" title=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" title=\"” after the the canonical param, but got „{}” instead.", header))?;
			let (title, next_header_segment) = next_header_segment.split_once("\" header=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" header=\"” after the the title param, but got „{}” instead.", header))?;
			let (header, next_header_segment) = next_header_segment.split_once("\" description=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" description=\"” after the the header param, but got „{}” instead.", header))?;
			let description = next_header_segment.strip_suffix("\">").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to end with a „\">” after the the description param, but got „.....{}” instead.", next_header_segment))?;
	
			return Ok((Metadata{lang, canonical, title, header, description}, content.join("\n")));
		} else {
			bail!("Not enough lines provided! Got {}, but expected at least 4.", lines.len());
		}
	}
}