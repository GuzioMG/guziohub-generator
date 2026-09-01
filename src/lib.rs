use anyhow::{Result, Context, bail, ensure};

pub fn process(filecontent: &String) -> Result<String>{
	let lines = filecontent.lines().collect::<Vec<&str>>();
	let meta = extract_metadata(lines.as_slice())?;
	dbg!(&meta);
	return Ok(meta.article);
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Metadata<'the_metadata_cannot_outlive_its_fields>{
	lang: &'the_metadata_cannot_outlive_its_fields str,
	canonical: &'the_metadata_cannot_outlive_its_fields str,
	title: &'the_metadata_cannot_outlive_its_fields str,
	header: &'the_metadata_cannot_outlive_its_fields str,
	description: &'the_metadata_cannot_outlive_its_fields str,

	article: String,
}

fn extract_metadata<'output_is_derived_from_input_so_it_cannot_outlive_input>(lines: &'output_is_derived_from_input_so_it_cannot_outlive_input [&str]) -> Result<Metadata<'output_is_derived_from_input_so_it_cannot_outlive_input>>{
	if let [doctype, header, content @ .., closing_tag] = lines {
		ensure!(doctype.starts_with("<!DOCTYPE ghtml-v1.0 \"") && doctype.ends_with("\">"), "Invalid G-HTML structure: Invalid doctype! Expected the 1st line to start with „<!DOCTYPE ghtml-v1.0 \"” and end with „\">”, but got „{}” instead.", doctype);
		ensure!(closing_tag.to_string() == "</html>", "Invalid G-HTML structure: No valid closing tag! Expected the last line to be „</html>”, but got „{}” instead.", closing_tag);
		
		let og_header = header;
		let header = header.strip_prefix("<html flavor=\"ghtml\" lang=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to start with „<html flavor=\"ghtml\" lang=\"”, but got „{}” instead.", og_header))?;
		let (lang, header) = header.split_once("\" canonical=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" canonical=\"” after the the lang param, but got „{}” instead.", og_header))?;
		let (canonical, header) = header.split_once("\" title=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" title=\"” after the the canonical param, but got „{}” instead.", og_header))?;
		let (title, header) = header.split_once("\" header=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" header=\"” after the the title param, but got „{}” instead.", og_header))?;
		let (article_header, header) = header.split_once("\" description=\"").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to have a „\" description=\"” after the the header param, but got „{}” instead.", og_header))?;		
		let description = header.strip_suffix("\">").with_context(|| format!("Invalid G-HTML structure: Invalid header: Expected the 2nd line to end with a „\">” after the the description param, but got „{}” instead.", og_header))?;

		return Ok(Metadata{
			lang, canonical, title,
			header: article_header,
			description,
			article: content.join("\n"),
		});
	} else {
		bail!("Not enough lines provided! Got {}, but expected at least 4.", lines.len());
	}
}