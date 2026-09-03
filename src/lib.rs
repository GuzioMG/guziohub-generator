use std::{env, fmt::{Display, Error}};

use anyhow::{Context, Result, bail, ensure};



pub fn process(filecontent: &String) -> Result<String> {
	let lines = filecontent.lines().collect::<Vec<&str>>();
	let meta = Metadata::new(lines.as_slice())?;
	dbg!(&meta.0);
	return Ok(meta.1);
}



#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Metadata<'metadata_fields> {
	lang: &'metadata_fields str,
	canonical: &'metadata_fields str,
	title: &'metadata_fields str,
	header: &'metadata_fields str,
	description: &'metadata_fields str,
}


impl<'output> Metadata<'output> {
	fn new<'inputs>(lines: &[&'inputs str]) -> Result<(Self, String)>
		where 'inputs: 'output,
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



#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Walker {
	index: usize,
	word_start: usize,
	word: Vec<WordSection>,
	active_line: Renderable<String>,
	past_lines: Vec<Renderable<String>>,
	active_tags: Vec<ParameterizedHtmlTag>,
	on: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Renderable<T> {
	length: usize,
	content: T,
}

type Complete = bool;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum WordSection {
	Indent(String),
	Literal(String),
	HtmlTag(HtmlTag, Complete),
	VarReplacement(String, Complete),
	HtmlEntity(String, Complete),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum HtmlTag {
	#[default] JustStarted,
	Opening(ParameterizedHtmlTag),
	Closing(String),
	SelfClosing(ParameterizedHtmlTag),
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ParameterizedHtmlTag {
	tag: String,
	args: Option<String>,
}


impl<T> Display for Renderable<T>
	where T: Display
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		return self.content.fmt(f);
	}
}

impl Display for WordSection {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		return match self.render() {
			Ok(result) => result.fmt(f),
			Err(_) => Err(Error)
		}
	}
}

impl Display for HtmlTag {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		return match self {
			HtmlTag::JustStarted => write!(f, "<"),
			HtmlTag::Closing(tag) => write!(f, "</{}>", tag),
			HtmlTag::Opening(tag) | HtmlTag::SelfClosing(tag) => {
				let ending = match self {
					HtmlTag::SelfClosing(_) => " />",
					_ => ">"
				};
				return match &tag.args {
					Some(args) => write!(f, "<{} {}{}", tag.tag, args, ending),
					None => write!(f, "<{}{}", tag.tag, ending),
				}
			}
		}
	}
}


impl WordSection {
	fn render(&self) -> Result<Renderable<String>> {
		return match self {
			WordSection::Indent(literal) | WordSection::Literal(literal) => Ok(Renderable{length: literal.chars().count(), content: literal.to_string()}),
			WordSection::HtmlTag(tag, true) => Ok(Renderable{length: 0, content: tag.to_string()}),
			WordSection::HtmlEntity(entity, true) => Ok(Renderable{length: 1, content: format!("&{};", entity)}),

			WordSection::VarReplacement(varname, true) => match env::var(varname) {
				Ok(content) => Ok(Renderable{length: content.chars().count(), content}),
				Err(err) => match err {
					env::VarError::NotPresent =>  bail!("Attempted to render an envar %{}% that doesn't exist!", varname),
					env::VarError::NotUnicode(os_string) => {
						let rs_string = os_string.to_string_lossy();
						return Ok(Renderable{length: rs_string.chars().count(), content: rs_string.to_string()});
					}
				}
			}

			WordSection::HtmlTag(_, false) | WordSection::VarReplacement(_, false) | WordSection::HtmlEntity(_, false) => bail!("Attempted to render an incomplete WordSection!")
		}
	}
}

impl Walker {
	fn walk(mut self) -> Result<Self> {
		if self.active_line.length == 0 && self.word.is_empty(){

		} else {
			
		}

		self.index+=1;
		return Ok(self);
	}
}